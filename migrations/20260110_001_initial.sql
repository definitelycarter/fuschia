-- Initial schema for Fuscia workflow execution storage

CREATE TABLE workflow_executions (
    execution_id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    status TEXT NOT NULL,
    config TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE workflow_tasks (
    task_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    status TEXT NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 1,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    output TEXT,
    error TEXT,
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(execution_id)
);

CREATE INDEX idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
CREATE INDEX idx_workflow_tasks_execution_id ON workflow_tasks(execution_id);
