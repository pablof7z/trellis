mod support;

use trellis_examples::mini_language_server::delete_file_showcase_trace;

fn main() {
    support::run("delete-file", delete_file_showcase_trace);
}
