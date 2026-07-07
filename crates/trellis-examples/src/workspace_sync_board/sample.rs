use super::types::{BoardColumn, IssueRecord, ProjectRecord, WorkspaceDataset};

impl WorkspaceDataset {
    /// Returns a deterministic sample dataset for tests and headless scripts.
    pub fn sample() -> Self {
        let projects = [
            ProjectRecord {
                id: "backend".to_owned(),
                org: "org-a".to_owned(),
                workspace: "workspace-a".to_owned(),
                name: "Backend".to_owned(),
                active: true,
            },
            ProjectRecord {
                id: "docs".to_owned(),
                org: "org-a".to_owned(),
                workspace: "workspace-a".to_owned(),
                name: "Docs".to_owned(),
                active: false,
            },
            ProjectRecord {
                id: "mobile".to_owned(),
                org: "org-b".to_owned(),
                workspace: "workspace-b".to_owned(),
                name: "Mobile".to_owned(),
                active: true,
            },
        ]
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

        let issues = [
            (
                "backend".to_owned(),
                vec![
                    IssueRecord {
                        id: "B-1".to_owned(),
                        project_id: "backend".to_owned(),
                        title: "Repair sync replay".to_owned(),
                        column: BoardColumn::Todo,
                        assignee: "alex".to_owned(),
                    },
                    IssueRecord {
                        id: "B-2".to_owned(),
                        project_id: "backend".to_owned(),
                        title: "Ship permission audit".to_owned(),
                        column: BoardColumn::Doing,
                        assignee: "casey".to_owned(),
                    },
                ],
            ),
            (
                "docs".to_owned(),
                vec![IssueRecord {
                    id: "D-1".to_owned(),
                    project_id: "docs".to_owned(),
                    title: "Refresh onboarding".to_owned(),
                    column: BoardColumn::Done,
                    assignee: "alex".to_owned(),
                }],
            ),
            (
                "mobile".to_owned(),
                vec![IssueRecord {
                    id: "M-1".to_owned(),
                    project_id: "mobile".to_owned(),
                    title: "Polish offline board".to_owned(),
                    column: BoardColumn::Doing,
                    assignee: "riley".to_owned(),
                }],
            ),
        ]
        .into_iter()
        .collect();

        let permissions = [(
            "alex".to_owned(),
            ["backend", "docs", "mobile"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )]
        .into_iter()
        .collect();

        Self {
            permissions,
            projects,
            issues,
        }
    }
}
