//! E2E tests for convergio-file-transport transfer CRUD and rsync helpers.

use convergio_db::pool::ConnPool;
use convergio_file_transport::transfer;
use convergio_file_transport::types::{
    TransferDirection, TransferRequest, TransferResult, TransferStatus,
};

fn setup_pool() -> ConnPool {
    let pool = convergio_db::pool::create_memory_pool().unwrap();
    let conn = pool.get().unwrap();
    for m in convergio_file_transport::schema::migrations() {
        conn.execute_batch(m.up).unwrap();
    }
    drop(conn);
    pool
}

fn sample_req() -> TransferRequest {
    TransferRequest {
        source_path: "/data/project/".into(),
        dest_path: "/backup/project/".into(),
        peer_name: "studio-mac".into(),
        ssh_target: "rob@192.168.1.50".into(),
        direction: TransferDirection::Push,
        exclude_patterns: vec![],
    }
}

fn sample_result() -> TransferResult {
    TransferResult {
        peer_name: "studio-mac".into(),
        bytes_transferred: 4096,
        files_count: 12,
        duration_ms: 350,
        status: TransferStatus::Success,
    }
}

fn seed_transfer(pool: &ConnPool) -> i64 {
    let conn = pool.get().unwrap();
    transfer::record_transfer(&conn, &sample_result(), &sample_req()).unwrap()
}

// --- Transfer record CRUD tests ---

#[tokio::test]
async fn record_and_retrieve_transfer() {
    let pool = setup_pool();
    let id = seed_transfer(&pool);
    assert!(id > 0);

    let conn = pool.get().unwrap();
    let rec = transfer::get_transfer(&conn, id).unwrap().unwrap();
    assert_eq!(rec.peer_name, "studio-mac");
    assert_eq!(rec.direction, "push");
    assert_eq!(rec.bytes_transferred, 4096);
    assert_eq!(rec.files_count, 12);
    assert_eq!(rec.status, "success");
    assert!(rec.error_message.is_none());
}

#[tokio::test]
async fn record_failed_transfer_stores_error() {
    let pool = setup_pool();
    let conn = pool.get().unwrap();
    let mut result = sample_result();
    result.status = TransferStatus::Failed("connection refused".into());
    let id = transfer::record_transfer(&conn, &result, &sample_req()).unwrap();

    let rec = transfer::get_transfer(&conn, id).unwrap().unwrap();
    assert_eq!(rec.status, "failed");
    assert_eq!(rec.error_message.as_deref(), Some("connection refused"));
}

#[tokio::test]
async fn record_partial_success_transfer() {
    let pool = setup_pool();
    let conn = pool.get().unwrap();
    let mut result = sample_result();
    result.status = TransferStatus::PartialSuccess("3 files skipped".into());
    let id = transfer::record_transfer(&conn, &result, &sample_req()).unwrap();

    let rec = transfer::get_transfer(&conn, id).unwrap().unwrap();
    assert_eq!(rec.status, "partial");
    assert_eq!(rec.error_message.as_deref(), Some("3 files skipped"));
}

#[tokio::test]
async fn list_transfers_empty() {
    let pool = setup_pool();
    let conn = pool.get().unwrap();
    let list = transfer::list_transfers(&conn, None, 50).unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn list_transfers_with_peer_filter() {
    let pool = setup_pool();
    seed_transfer(&pool);

    let conn = pool.get().unwrap();
    let mut other_req = sample_req();
    other_req.peer_name = "linux-box".into();
    let mut other_result = sample_result();
    other_result.peer_name = "linux-box".into();
    transfer::record_transfer(&conn, &other_result, &other_req).unwrap();

    let all = transfer::list_transfers(&conn, None, 50).unwrap();
    assert_eq!(all.len(), 2);

    let filtered = transfer::list_transfers(&conn, Some("studio-mac"), 50).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].peer_name, "studio-mac");

    let empty = transfer::list_transfers(&conn, Some("nonexistent"), 50).unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn list_transfers_respects_limit() {
    let pool = setup_pool();
    for _ in 0..5 {
        seed_transfer(&pool);
    }
    let conn = pool.get().unwrap();
    let limited = transfer::list_transfers(&conn, None, 3).unwrap();
    assert_eq!(limited.len(), 3);
}

#[tokio::test]
async fn get_transfer_not_found() {
    let pool = setup_pool();
    let conn = pool.get().unwrap();
    let rec = transfer::get_transfer(&conn, 999).unwrap();
    assert!(rec.is_none());
}

// --- Rsync command builder tests ---

#[test]
fn rsync_command_push_structure() {
    let req = sample_req();
    let cmd = convergio_file_transport::rsync::build_rsync_command(&req, "rob@192.168.1.50");
    let prog = cmd.as_std().get_program().to_str().unwrap();
    assert_eq!(prog, "rsync");
    let args: Vec<_> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"-avz"));
    assert!(args.contains(&"--delete"));
    assert!(args.contains(&"/data/project/"));
    assert!(args.contains(&"rob@192.168.1.50:/backup/project/"));
}

#[test]
fn rsync_command_pull_structure() {
    let mut req = sample_req();
    req.direction = TransferDirection::Pull;
    let cmd = convergio_file_transport::rsync::build_rsync_command(&req, "rob@192.168.1.50");
    let args: Vec<_> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"rob@192.168.1.50:/data/project/"));
    assert!(args.contains(&"/backup/project/"));
}

#[test]
fn rsync_command_with_excludes() {
    let mut req = sample_req();
    req.exclude_patterns = vec!["*.tmp".into(), ".git".into()];
    let cmd = convergio_file_transport::rsync::build_rsync_command(&req, "rob@192.168.1.50");
    let args: Vec<_> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"--exclude=*.tmp"));
    assert!(args.contains(&"--exclude=.git"));
}

#[test]
fn rsync_output_parsing() {
    let output = "\
file1.txt
file2.txt
sent 1,234 bytes  received 56 bytes  2,580.00 bytes/sec
total size is 5,678  speedup is 4.40";
    let (bytes, files) = convergio_file_transport::rsync::parse_rsync_output(output);
    assert_eq!(bytes, 1234);
    assert_eq!(files, 2);
}

#[test]
fn rsync_output_parsing_empty() {
    let (bytes, files) = convergio_file_transport::rsync::parse_rsync_output("");
    assert_eq!(bytes, 0);
    assert_eq!(files, 0);
}
