//! The inventory block in `docs/REPO.tsare` is what this build prints.
//!
//! The block is generated, not written: the commands come from the dispatch table, the codes from
//! the explanation table, the documents from the fixture directory.  A hand-written copy would be
//! a snapshot of what somebody remembered, and the claims that range over it would quietly stop
//! being about this build.  This test is the thing that makes the block live.

use std::path::Path;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

const START: &str = "# --- inventory:";
const END: &str = "# --- end inventory ---\n";

#[test]
fn the_inventory_block_is_what_the_build_prints() {
    let output = Command::new(binary())
        .args(["inventory", "--fixtures", "tests/fixtures"])
        .output()
        .expect("run the binary");
    assert!(output.status.success(), "{output:?}");
    let printed = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/REPO.tsare");
    let repo = std::fs::read_to_string(&path)
        .expect("docs/REPO.tsare is part of the repository")
        .replace("\r\n", "\n");
    let start = repo.find(START).expect("the inventory block is marked");
    let end = repo.find(END).expect("the inventory block is closed") + END.len();
    assert_eq!(
        &repo[start..end],
        printed,
        "the block drifted from what the build prints; run `tsare inventory --fixtures tests/fixtures` and paste it"
    );
}

#[test]
fn the_block_names_this_build_s_tables() {
    let output = Command::new(binary())
        .args(["inventory", "--fixtures", "tests/fixtures", "--json"])
        .output()
        .expect("run the binary");
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(text.starts_with("{\"commands\":"), "{text}");
    assert!(
        text.contains("\"inventory\""),
        "the command lists itself: {text}"
    );
    assert!(text.contains("\"E-COLLAPSE\""), "{text}");
    assert!(text.contains("\"hello.tsare\""), "{text}");
}
