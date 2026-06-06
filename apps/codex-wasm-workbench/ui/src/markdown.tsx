import type { ComponentProps } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import 'highlight.js/styles/github-dark.css';

/**
 * Renders assistant-facing markdown with open-source markdown and syntax
 * highlighting libraries instead of a Workbench-owned markdown parser.
 *
 * The Workbench is an application-owned UI surface, so this component acts as a
 * presentation Adapter: it receives already-sanitized execution-event text from
 * the application execution service and delegates Markdown/GFM parsing to
 * `react-markdown`, GFM table parsing to `remark-gfm`, and code highlighting to
 * `rehype-highlight`. Keeping parsing out of Macaca OS code preserves the
 * microkernel boundary while still giving the app a richer user experience.
 */
export function MarkdownView({ markdown }: { markdown: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        components={{
          a: MarkdownLink,
          table: MarkdownTable,
        }}
        rehypePlugins={[rehypeHighlight]}
        remarkPlugins={[remarkGfm]}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
}

function MarkdownLink({ children, href, ...props }: ComponentProps<'a'>) {
  return (
    <a href={href} rel="noreferrer" target="_blank" {...props}>
      {children}
    </a>
  );
}

function MarkdownTable({ children, ...props }: ComponentProps<'table'>) {
  return (
    <div className="markdown-table-scroll" role="region" aria-label="Markdown table">
      <table {...props}>{children}</table>
    </div>
  );
}
