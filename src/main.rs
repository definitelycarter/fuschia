use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;

use fuschia_component_registry::FsComponentRegistry;
use fuschia_config::WorkflowDef;
use fuschia_engine::{EngineConfig, WorkflowEngine};
use fuschia_resolver::{Resolver, StandardResolver};

/// Fuscia - A workflow engine built on WebAssembly components
#[derive(Parser)]
#[command(name = "fuschia")]
#[command(version, about, long_about = None)]
struct Cli {
  /// Path to the data directory (default: ~/.fuschia)
  #[arg(long, global = true)]
  data_dir: Option<PathBuf>,

  #[command(subcommand)]
  command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
  /// Run a workflow or task
  Run {
    #[command(subcommand)]
    target: RunTarget,
  },
}

#[derive(Subcommand)]
enum RunTarget {
  /// Run an entire workflow
  Workflow {
    /// Path to the workflow file (JSON or YAML)
    workflow_file: PathBuf,
  },

  /// Run a single task from a workflow
  Task {
    /// Path to the workflow file (JSON or YAML)
    workflow_file: PathBuf,

    /// The node ID to execute
    #[arg(long)]
    node: String,
  },
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  let data_dir = cli.data_dir.unwrap_or_else(|| {
    dirs::home_dir()
      .expect("could not determine home directory")
      .join(".fuschia")
  });

  match cli.command {
    Some(Commands::Run { target }) => match target {
      RunTarget::Workflow { workflow_file } => {
        run_workflow(workflow_file, data_dir)?;
      }
      RunTarget::Task {
        workflow_file,
        node,
      } => {
        run_task(workflow_file, node, data_dir)?;
      }
    },
    None => {
      println!("fuschia - use --help to see available commands");
    }
  }

  Ok(())
}

fn run_workflow(workflow_file: PathBuf, data_dir: PathBuf) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async { run_workflow_async(workflow_file, data_dir).await })
}

async fn run_workflow_async(workflow_file: PathBuf, data_dir: PathBuf) -> Result<()> {
  // Read workflow definition
  let workflow_content = tokio::fs::read_to_string(&workflow_file)
    .await
    .with_context(|| format!("failed to read workflow file: {}", workflow_file.display()))?;

  let workflow_def: WorkflowDef = serde_json::from_str(&workflow_content)
    .with_context(|| format!("failed to parse workflow file: {}", workflow_file.display()))?;

  eprintln!("Loaded workflow: {}", workflow_def.name);

  // Read payload from stdin
  let payload = read_payload_from_stdin()?;
  eprintln!("Payload: {}", payload);

  // Set up component registry
  let components_dir = data_dir.join("components");
  let registry = FsComponentRegistry::new(&components_dir);

  // Resolve workflow
  let resolver = StandardResolver::new(registry);
  let workflow = resolver
    .resolve(workflow_def)
    .await
    .context("failed to resolve workflow")?;

  eprintln!("Resolved workflow with {} nodes", workflow.nodes.len());

  // Create engine
  let config = EngineConfig {
    component_base_path: components_dir,
  };
  let engine = WorkflowEngine::new(config).context("failed to create workflow engine")?;

  // Execute workflow
  let cancel = CancellationToken::new();
  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .context("workflow execution failed")?;

  eprintln!("Execution completed: {}", result.execution_id);
  eprintln!("Nodes executed: {}", result.node_results.len());

  // Print results as JSON
  let output: serde_json::Map<String, serde_json::Value> = result
    .node_results
    .into_iter()
    .map(|(id, r)| (id, r.data))
    .collect();

  println!("{}", serde_json::to_string_pretty(&output)?);

  Ok(())
}

fn run_task(workflow_file: PathBuf, node: String, data_dir: PathBuf) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async { run_task_async(workflow_file, node, data_dir).await })
}

async fn run_task_async(workflow_file: PathBuf, node_id: String, data_dir: PathBuf) -> Result<()> {
  // Read workflow definition
  let workflow_content = tokio::fs::read_to_string(&workflow_file)
    .await
    .with_context(|| format!("failed to read workflow file: {}", workflow_file.display()))?;

  let workflow_def: WorkflowDef = serde_json::from_str(&workflow_content)
    .with_context(|| format!("failed to parse workflow file: {}", workflow_file.display()))?;

  // Find the node in the workflow
  let node_def = workflow_def
    .nodes
    .iter()
    .find(|n| n.node_id == node_id)
    .with_context(|| format!("node '{}' not found in workflow", node_id))?;

  eprintln!("Running node: {} (type: {:?})", node_id, node_def.node_type);

  // Read payload from stdin
  let payload = read_payload_from_stdin()?;
  eprintln!("Payload: {}", payload);

  // Set up component registry
  let components_dir = data_dir.join("components");
  let registry = FsComponentRegistry::new(&components_dir);

  // Resolve workflow
  let resolver = StandardResolver::new(registry);
  let workflow = resolver
    .resolve(workflow_def)
    .await
    .context("failed to resolve workflow")?;

  // Find the resolved node
  let resolved_node = workflow
    .nodes
    .get(&node_id)
    .with_context(|| format!("resolved node '{}' not found", node_id))?;

  // Create engine
  let config = EngineConfig {
    component_base_path: components_dir,
  };
  let engine = WorkflowEngine::new(config).context("failed to create workflow engine")?;

  // Execute just this node
  let cancel = CancellationToken::new();
  let result = engine
    .execute_node(resolved_node, payload, cancel)
    .await
    .context("node execution failed")?;

  eprintln!("Node execution completed");

  // Print result as JSON
  println!("{}", serde_json::to_string_pretty(&result.data)?);

  Ok(())
}

fn read_payload_from_stdin() -> Result<serde_json::Value> {
  use std::io::IsTerminal;

  if io::stdin().is_terminal() {
    // No stdin pipe, use empty object
    Ok(serde_json::json!({}))
  } else {
    // Read from stdin
    let mut input = String::new();
    io::stdin()
      .read_to_string(&mut input)
      .context("failed to read payload from stdin")?;

    if input.trim().is_empty() {
      Ok(serde_json::json!({}))
    } else {
      serde_json::from_str(&input).context("failed to parse payload JSON from stdin")
    }
  }
}
