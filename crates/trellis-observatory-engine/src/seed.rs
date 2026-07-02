use std::collections::BTreeMap;

use crate::types::{CanonicalInputs, CompilerConfig, FileRecord};

pub fn branch_files(branch: &str) -> BTreeMap<String, FileRecord> {
    match branch {
        "feature/schema-v2" => files(&[
            ("src/app.tl", APP_V2, false),
            ("src/math.tl", MATH, false),
            ("src/schema_v2.tl", SCHEMA_V2, false),
            ("generated/api_schema.tl", API_SCHEMA, true),
        ]),
        _ => files(&[
            ("src/app.tl", APP_MAIN, false),
            ("src/legacy_user.tl", LEGACY, false),
            ("src/math.tl", MATH, false),
            ("src/schema.tl", SCHEMA, false),
            ("generated/api_schema.tl", API_SCHEMA, true),
        ]),
    }
}

pub fn initial_inputs() -> CanonicalInputs {
    CanonicalInputs {
        active_branch: "main".to_owned(),
        files: branch_files("main"),
        open_editors: vec!["src/app.tl".to_owned()],
        active_editor: Some("src/app.tl".to_owned()),
        compiler_config: CompilerConfig::Strict,
        generated_files_enabled: true,
        host_statuses: Vec::new(),
        scenario_revision: 1,
    }
}

fn files(entries: &[(&str, &str, bool)]) -> BTreeMap<String, FileRecord> {
    entries
        .iter()
        .map(|(path, contents, generated)| {
            (
                (*path).to_owned(),
                FileRecord {
                    path: (*path).to_owned(),
                    contents: (*contents).to_owned(),
                    generated: *generated,
                },
            )
        })
        .collect()
}

pub const APP_MAIN: &str = r#"module app

import "./math.tl"
import "./legacy_user.tl"
import "./schema.tl"

let user = load_user()
let total = add(1, "two")
user.email_verified
"#;

pub const APP_V2: &str = r#"module app

import "./math.tl"
import "./schema_v2.tl"

let user = load_user()
let total = add(1, 2)
user.email_verified
"#;

const LEGACY: &str = r#"module legacy_user

let name = "Ada"
TODO_ERROR
"#;

const MATH: &str = r#"module math

fn add(a: number, b: number) -> number
"#;

const SCHEMA: &str = r#"module schema

type User {
  id: string
  email: string
}
"#;

pub const SCHEMA_V2: &str = r#"module schema_v2

type User {
  id: string
  email: string
  email_verified: bool
}
"#;

const API_SCHEMA: &str = r#"module api_schema

type ApiUser {
  id: string
  email: string
}
"#;
