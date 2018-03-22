// Inline common build protocols behavior
include!("../libbuild-protocols.rs");

fn main() {
    protocols::generate_protocols();
}
