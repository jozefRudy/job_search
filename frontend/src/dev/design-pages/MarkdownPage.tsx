import { Card } from "~/components/ui/Card";
import { Stack } from "~/components/ui/layout/Stack";
import { Markdown } from "~/components/ui/Markdown";
import { DevLayout } from "../DevLayout";

const sampleText = `# Heading

This is a paragraph with **bold**, *italic*, and [a link](https://example.com).

## Ordered list

1. First item
2. Second item
3. Third item

## Unordered list

- Alpha
- Beta
- Gamma

Inline code: \`cargo build\`.
`;

export default function MarkdownPage() {
  return (
    <DevLayout title="Markdown" backHref="/dev">
      <Stack gap="lg">
        <Card>
          <Card.Body>
            <Stack gap="md">
              <h2 class="font-semibold text-lg">Small (default)</h2>
              <Markdown text={sampleText} />
            </Stack>
          </Card.Body>
        </Card>

        <Card>
          <Card.Body>
            <Stack gap="md">
              <h2 class="font-semibold text-lg">Medium</h2>
              <Markdown text={sampleText} size="md" />
            </Stack>
          </Card.Body>
        </Card>

        <Card>
          <Card.Body>
            <Stack gap="md">
              <h2 class="font-semibold text-lg">Large</h2>
              <Markdown text={sampleText} size="lg" />
            </Stack>
          </Card.Body>
        </Card>
      </Stack>
    </DevLayout>
  );
}
