import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface MarkdownContentProps {
  content: string;
}

export function MarkdownContent({ content }: MarkdownContentProps) {
  return (
    <div className="markdown-content">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          a({ href, children }) {
            const external = href?.startsWith("https://") || href?.startsWith("http://");
            if (!external) {
              return (
                <span className="markdown-local-reference" title={href || undefined}>
                  {children}
                </span>
              );
            }
            return (
              <a href={href} target="_blank" rel="noreferrer noopener">
                {children}
              </a>
            );
          },
          img({ alt }) {
            return <span className="markdown-image-reference">Immagine: {alt || "allegato"}</span>;
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
