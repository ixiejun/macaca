import { MarkdownView } from './markdown';
import { presentTimelineEvent } from './presenter';
import type { TimelineEntry } from './types';

/**
 * Render the application-owned execution timeline.
 *
 * Macaca stores provider-neutral events and streams them to the app.  This
 * component deliberately owns only presentation: it turns already-sanitized
 * event data into readable cards without introducing app-specific behavior into
 * Macaca OS services or the host frontend shell.
 */
export function Timeline({ events }: { events: TimelineEntry[] }) {
  if (events.length === 0) {
    return <div className="event-timeline"><p className="empty">Submit a task to stream execution events.</p></div>;
  }
  return (
    <>
      <TimelineOverview events={events} />
      <div className="event-timeline">{events.map((entry, index) => <TimelineRow entry={entry} events={events} index={index} key={`${entry.type}:${entry.at}:${index}`} />)}</div>
    </>
  );
}

function TimelineOverview({ events }: { events: TimelineEntry[] }) {
  const counts = timelineCounts(events);
  const items = [
    ['Events', counts.total],
    ['Agent turns', counts.agentTurns],
    ['Tool activity', counts.toolActivity],
    ['Errors', counts.errors],
    ['Completion', counts.completions],
  ];
  return (
    <section className="timeline-overview" aria-label="Execution overview">
      {items.map(([label, value]) => (
        <div className="timeline-overview-item" key={label}>
          <span>{label}</span>
          <strong>{value}</strong>
        </div>
      ))}
    </section>
  );
}

function TimelineRow({ entry, events, index }: { entry: TimelineEntry; events: TimelineEntry[]; index: number }) {
  const section = timelineSection(entry);
  const previousSection = index > 0 ? timelineSection(events[index - 1]) : null;
  return (
    <>
      {section !== previousSection && <TimelineSectionHeader section={section} />}
      <TimelineCard entry={entry} />
    </>
  );
}

function TimelineSectionHeader({ section }: { section: string }) {
  return (
    <div className="event-section-header">
      <span>{section}</span>
    </div>
  );
}

function TimelineCard({ entry }: { entry: TimelineEntry }) {
  const view = presentTimelineEvent(entry);
  const tone = view.tone || 'neutral';
  return (
    <article className={`event-card event-tone-${tone} event-${entry.type.replaceAll('_', '-')}`}>
      <div className="event-card-header">
        <span className="event-state-dot" aria-hidden="true" />
        <div className="event-title-stack">
          <p className="event-kicker">{entryLabel(entry.type)}</p>
          <h3 className="event-title">{view.title}</h3>
        </div>
      </div>
      {view.summary && <p className="event-summary">{view.summary}</p>}
      <EventBody view={view} />
      {view.meta.length > 0 && <ul className="event-meta">{view.meta.map((item) => <li key={item}>{item}</li>)}</ul>}
      {view.details && view.details.length > 0 && (
        <details className="event-details">
          <summary>Trace details</summary>
          <ul>{view.details.map((item) => <li key={item}>{item}</li>)}</ul>
        </details>
      )}
    </article>
  );
}

function EventBody({ view }: { view: ReturnType<typeof presentTimelineEvent> }) {
  if (!view.body) return null;
  if (view.format === 'markdown') return <div className="event-body"><MarkdownView markdown={view.body} /></div>;
  if (view.usePre) return <pre className="event-body-pre">{view.body}</pre>;
  return <p className="event-body-text">{view.body}</p>;
}

function entryLabel(type: string): string {
  return type.replaceAll('_', ' ').replace(/\b\w/g, (char) => char.toUpperCase());
}

function timelineSection(entry: TimelineEntry): string {
  if (entry.type === 'stream_connected' || entry.type === 'model_catalog' || entry.type === 'model_route' || entry.type === 'execution_start_result') return 'Session setup';
  if (entry.type === 'tool_result') return 'Tool activity';
  if (entry.type !== 'execution_event') return 'Workbench activity';
  const event = entry.data && typeof entry.data === 'object' && !Array.isArray(entry.data) ? entry.data as { event_type?: string; sanitized_payload?: { data?: Record<string, unknown> | null } } : {};
  const eventType = event.event_type || '';
  const toolName = typeof event.sanitized_payload?.data?.tool_name === 'string' ? event.sanitized_payload.data.tool_name : '';
  if (eventType.includes('Tool') || toolName) return 'Tool activity';
  if (eventType.includes('Llm') || eventType.includes('ProviderHeartbeat')) return 'Agent turns';
  if (eventType.includes('Completed') || eventType.includes('Failed')) return 'Completion';
  if (eventType.includes('Provider') || eventType.includes('Execution')) return 'Run setup';
  return 'Execution events';
}

function timelineCounts(events: TimelineEntry[]) {
  return events.reduce((counts, entry) => {
    const section = timelineSection(entry);
    counts.total += 1;
    if (section === 'Agent turns') counts.agentTurns += 1;
    if (section === 'Tool activity') counts.toolActivity += 1;
    if (entryHasError(entry)) counts.errors += 1;
    if (section === 'Completion') counts.completions += 1;
    return counts;
  }, { total: 0, agentTurns: 0, toolActivity: 0, errors: 0, completions: 0 });
}

function entryHasError(entry: TimelineEntry): boolean {
  if (entry.type === 'bridge_error' || entry.type === 'error') return true;
  if (!entry.data || typeof entry.data !== 'object' || Array.isArray(entry.data)) return false;
  const data = entry.data as { event_type?: string; sanitized_payload?: { data?: Record<string, unknown> | null } };
  return data.event_type === 'ExecutionFailed' || data.sanitized_payload?.data?.is_error === true;
}
