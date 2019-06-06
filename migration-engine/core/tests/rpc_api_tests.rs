#![allow(non_snake_case)]
mod test_harness;
use migration_core::rpc_api::RpcApi;
use test_harness::*;

#[test]
fn simple_end_to_end_test() {
    let json = r#"
        {
            "id": 1,
            "jsonrpc": "2.0",
            "method": "listMigrations",
            "params": {
                "projectInfo": "the-project-id"
            }
        }
    "#;

    let result = handle_command(&json);
    assert_eq!(result, r#"{"jsonrpc":"2.0","result":[],"id":1}"#);
}

fn handle_command(command: &str) -> String {
    // just using this because of its feature to reset the test db
    run_test_with_engine(|_| {
        let rpc_api = RpcApi::new();
        rpc_api.handle_input(command)
    })
}