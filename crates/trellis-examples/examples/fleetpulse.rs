mod support;

use trellis_examples::telemetry_dashboard::revoke_permission_showcase_trace;

fn main() {
    support::run("revoke-permission", revoke_permission_showcase_trace);
}
