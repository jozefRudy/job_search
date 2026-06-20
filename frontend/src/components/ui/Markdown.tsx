import DOMPurify from "dompurify";
import { marked } from "marked";
import type { Size } from "~/components/ui/layout/layout";
import { cn } from "~/lib/utils";

DOMPurify.setConfig({ ADD_ATTR: ["target", "rel"] });
DOMPurify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "A") {
    node.setAttribute("target", "_blank");
    node.setAttribute("rel", "noopener noreferrer");
  }
});

export interface MarkdownProps {
  text: string;
  size?: Size;
  class?: string;
}

const sizeMap: Record<Size, string> = {
  sm: "prose-sm",
  md: "",
  lg: "prose-lg",
};

export function Markdown(props: MarkdownProps) {
  const html = () => {
    const parsed = marked.parse(props.text, { async: false }) as string;
    return DOMPurify.sanitize(parsed);
  };
  return (
    <div
      class={cn("prose max-w-none", sizeMap[props.size ?? "sm"], props.class)}
      innerHTML={html()}
    />
  );
}
