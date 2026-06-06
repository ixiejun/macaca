import type { AgentStatusItem, TaskBoardItem } from './types';

type CollaborationPanelProps = {
  tasks: TaskBoardItem[];
  agents: AgentStatusItem[];
};

/**
 * Render Macaca-owned task and agent collaboration state.
 *
 * This component is intentionally presentation-only.  It does not infer tasks
 * from execution events and it does not synthesize agent state from local UI
 * activity; Workbench receives both collections from Macaca's generic shell
 * bridge, which in turn reads the Task Service and kernel agent status views.
 */
export function CollaborationPanel({ tasks, agents }: CollaborationPanelProps) {
  return (
    <section className="collaboration-panel" aria-label="Macaca collaboration state">
      <div className="panel-header compact-header">
        <h2>Collaboration</h2>
        <span>{tasks.length} tasks · {agents.length} agents</span>
      </div>
      <TaskBoard tasks={tasks} />
      <AgentStatusList agents={agents} />
    </section>
  );
}

function TaskBoard({ tasks }: { tasks: TaskBoardItem[] }) {
  if (tasks.length === 0) {
    return (
      <div className="collaboration-empty">
        <strong>Task board is empty</strong>
        <span>Macaca Task Service has not published tasks for this session yet.</span>
      </div>
    );
  }
  return (
    <div className="collaboration-section">
      <h3>Tasks</h3>
      <ol className="task-board-list">
        {tasks.map((task, index) => (
          <li key={taskId(task) || `${task.title || 'task'}-${index}`}>
            <div className="task-row-title">
              <strong>{task.title || 'Untitled task'}</strong>
              <span className={`state-chip ${statusClass(task.status)}`}>{task.status || 'Unknown'}</span>
            </div>
            <p>{task.description || 'No task description was provided by the task service.'}</p>
            <div className="task-row-meta">
              <span>Agent: {task.assigned_agent || 'unassigned'}</span>
              <span>Priority: {task.priority ?? 'n/a'}</span>
              <span>Seq: {task.sequence_number ?? index + 1}</span>
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}

function AgentStatusList({ agents }: { agents: AgentStatusItem[] }) {
  if (agents.length === 0) {
    return (
      <div className="collaboration-empty">
        <strong>No agents loaded</strong>
        <span>The application metadata did not return app-scoped agents.</span>
      </div>
    );
  }
  return (
    <div className="collaboration-section">
      <h3>Agents</h3>
      <ul className="agent-status-list">
        {agents.map((agent) => {
          const activity = agent.activity?.type || (agent.is_active ? 'running' : 'idle');
          return (
            <li key={agent.id || agent.name}>
              <div className="agent-row-main">
                <strong>{agent.name || 'Unnamed agent'}</strong>
                <span className={`state-chip ${statusClass(activity)}`}>{activity}</span>
              </div>
              <p>{agent.activity?.context || agent.current_task || agent.state || 'No active task reported.'}</p>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function taskId(task: TaskBoardItem): string {
  if (typeof task.id === 'string') return task.id;
  if (task.id && typeof task.id === 'object') {
    const record = task.id as Record<string, unknown>;
    if (typeof record.value === 'string') return record.value;
    if (typeof record['0'] === 'string') return record['0'];
  }
  return '';
}

function statusClass(status?: string): string {
  const normalized = String(status || '').toLowerCase();
  if (normalized.includes('complete')) return 'is-success';
  if (normalized.includes('fail') || normalized.includes('error')) return 'is-danger';
  if (normalized.includes('progress') || normalized.includes('working') || normalized.includes('thinking')) return 'is-progress';
  if (normalized.includes('pending') || normalized.includes('assigned')) return 'is-warning';
  return 'is-neutral';
}
