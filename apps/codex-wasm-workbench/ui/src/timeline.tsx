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
  return <div className="event-timeline">{events.map((entry, index) => <TimelineCard entry={entry} key={`${entry.type}:${entry.at}:${index}`} />)}</div>;
}

function TimelineCard({ entry }: { entry: TimelineEntry }) {
  const view = presentTimelineEvent(entry);
  const tone = view.tone || 'neutral';
  return (
    <article className={`event-card event-tone-${tone} event-${entry.type.replaceAll('_', '-')}`}>
      <div className="event-card-header">
        <span className="event-state-dot" aria-hidden="true" />
        <div>
          <p className="event-kicker">{entryLabel(entry.type)}</p>
          <h3>{view.title}</h3>
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
