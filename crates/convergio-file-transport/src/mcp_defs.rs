//! MCP tool definitions for the file-transport extension.

use convergio_types::extension::McpToolDef;
use serde_json::json;

pub fn file_transport_tools() -> Vec<McpToolDef> {
    vec![
        McpToolDef {
            name: "cvg_file_push".into(),
            description: "Push files to a remote peer.".into(),
            method: "POST".into(),
            path: "/api/file-transport/push".into(),
            input_schema: json!({"type": "object", "properties": {"peer_id": {"type": "string"}, "path": {"type": "string"}}, "required": ["peer_id", "path"]}),
            min_ring: "trusted".into(),
            path_params: vec![],
        },
        McpToolDef {
            name: "cvg_file_pull".into(),
            description: "Pull files from a remote peer.".into(),
            method: "POST".into(),
            path: "/api/file-transport/pull".into(),
            input_schema: json!({"type": "object", "properties": {"peer_id": {"type": "string"}, "path": {"type": "string"}}, "required": ["peer_id", "path"]}),
            min_ring: "trusted".into(),
            path_params: vec![],
        },
        McpToolDef {
            name: "cvg_list_transfers".into(),
            description: "List file transfers.".into(),
            method: "GET".into(),
            path: "/api/file-transport/transfers".into(),
            input_schema: json!({"type": "object", "properties": {}}),
            min_ring: "community".into(),
            path_params: vec![],
        },
        McpToolDef {
            name: "cvg_get_transfer".into(),
            description: "Get details of a file transfer.".into(),
            method: "GET".into(),
            path: "/api/file-transport/transfers/:id".into(),
            input_schema: json!({"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}),
            min_ring: "community".into(),
            path_params: vec!["id".into()],
        },
    ]
}
