#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Working,
    Blocked,
    Done,
}

impl AgentState {
    pub fn icon(self) -> &'static str {
        match self {
            AgentState::Idle => "●",
            AgentState::Working => "◐",
            AgentState::Blocked => "⚠",
            AgentState::Done => "✓",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AgentState::Idle => "idle",
            AgentState::Working => "working",
            AgentState::Blocked => "blocked",
            AgentState::Done => "done",
        }
    }
}

pub struct MockPane {
    pub agent_name: &'static str,
    pub state: AgentState,
    pub content: Vec<&'static str>,
}

pub struct MockTab {
    pub name: &'static str,
    pub panes: Vec<MockPane>,
}

pub struct MockWorkspace {
    pub name: &'static str,
    pub branch: &'static str,
    pub ahead: u32,
    pub behind: u32,
    pub tabs: Vec<MockTab>,
}

pub fn mock_workspaces() -> Vec<MockWorkspace> {
    vec![
        MockWorkspace {
            name: "sieg",
            branch: "master",
            ahead: 2,
            behind: 0,
            tabs: vec![
                MockTab {
                    name: "editor",
                    panes: vec![
                        MockPane {
                            agent_name: "claude",
                            state: AgentState::Working,
                            content: vec![
                                "$ cargo build",
                                "   Compiling sieg v0.1.0",
                                "    Finished dev [unoptimized] target(s) in 4.2s",
                                "",
                                "> refactoring sidebar token layout...",
                                "> editing src/ui/sidebar/tokens.rs",
                            ],
                        },
                        MockPane {
                            agent_name: "shell",
                            state: AgentState::Idle,
                            content: vec!["$ just check", "waiting..."],
                        },
                    ],
                },
                MockTab {
                    name: "tests",
                    panes: vec![MockPane {
                        agent_name: "codex",
                        state: AgentState::Blocked,
                        content: vec![
                            "running cargo nextest...",
                            "",
                            "FAIL sidebar::tests::token_budget_shrinks",
                            "",
                            "needs input: overwrite fixture? [y/n]",
                        ],
                    }],
                },
            ],
        },
        MockWorkspace {
            name: "rhema-app",
            branch: "feature/onboarding",
            ahead: 0,
            behind: 1,
            tabs: vec![MockTab {
                name: "main",
                panes: vec![MockPane {
                    agent_name: "claude",
                    state: AgentState::Done,
                    content: vec![
                        "> onboarding flow updated",
                        "> all tests passing",
                        "",
                        "$ ",
                    ],
                }],
            }],
        },
        MockWorkspace {
            name: "docs-site",
            branch: "main",
            ahead: 0,
            behind: 0,
            tabs: vec![MockTab {
                name: "shell",
                panes: vec![MockPane {
                    agent_name: "shell",
                    state: AgentState::Idle,
                    content: vec!["$ "],
                }],
            }],
        },
    ]
}
