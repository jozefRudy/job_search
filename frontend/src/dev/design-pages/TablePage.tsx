import { A } from "@solidjs/router";
import { createSignal } from "solid-js";
import { Button } from "~/components/ui/Button";
import type { ColumnDef, TableLoadState } from "~/components/ui/data/Table";
import { Table } from "~/components/ui/data/Table";
import { Container } from "~/components/ui/layout/Container";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { formatNumber } from "~/lib/utils";
import { DevLayout } from "../DevLayout";

interface Post {
  id: number;
  title: string;
  subreddit: string;
  score: number;
  comments: number;
  date: string;
}

const posts: Post[] = [
  {
    id: 1,
    title: "AI automation tips",
    subreddit: "r/SaaS",
    score: 342,
    comments: 56,
    date: "2025-05-08",
  },
  {
    id: 2,
    title: "Marketing on Reddit",
    subreddit: "r/marketing",
    score: 128,
    comments: 23,
    date: "2025-05-07",
  },
  {
    id: 3,
    title: "Open source tools",
    subreddit: "r/webdev",
    score: 891,
    comments: 104,
    date: "2025-05-06",
  },
  {
    id: 4,
    title: "Side project launch",
    subreddit: "r/SideProject",
    score: 67,
    comments: 12,
    date: "2025-05-05",
  },
  {
    id: 5,
    title: "LLM cost optimization",
    subreddit: "r/MachineLearning",
    score: 12847,
    comments: 1023,
    date: "2025-05-04",
  },
];

const baseColumns: ColumnDef<Post>[] = [
  { key: "title", header: "Title" },
  { key: "subreddit", header: "Subreddit" },
  {
    key: "score",
    header: "Score",
    class: "text-right",
    format: (v) => formatNumber(Number(v)),
  },
  {
    key: "comments",
    header: "Comments",
    class: "text-right",
    format: (v) => formatNumber(Number(v)),
  },
  { key: "date", header: "Date" },
];

const actionColumns: ColumnDef<Post>[] = [
  ...baseColumns.slice(0, 3),
  {
    key: "actions",
    header: "",
    cell: () => (
      <Row gap="sm">
        <Button variant="ghost" size="sm" icon="edit" />
        <Button variant="ghost" size="sm" icon="delete" />
      </Row>
    ),
  },
];

export default function TablePage() {
  const [loadState, setLoadState] = createSignal<TableLoadState>("normal");
  const [empty, setEmpty] = createSignal(false);

  const data = () => (empty() ? [] : posts);

  return (
    <DevLayout title="Table Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Default</h2>
          <Table columns={baseColumns} data={posts} />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Zebra</h2>
          <Table columns={baseColumns} data={posts} zebra />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Hoverable</h2>
          <Table columns={baseColumns} data={posts} hoverable />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Sizes</h2>
          <Stack gap="lg">
            {(["sm", "md", "lg"] as const).map((size) => (
              <Stack gap="sm">
                <h3 class="font-medium text-base-content/60 text-sm">{size}</h3>
                <Table columns={baseColumns} data={posts} size={size} zebra />
              </Stack>
            ))}
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Constrained Width</h2>
          <p class="text-base-content/60 text-sm">
            Table inside narrow container — columns compress, horizontal scroll
            activates.
          </p>
          <Container maxWidth="sm">
            <Table columns={baseColumns} data={posts} zebra />
          </Container>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Clickable Titles</h2>
          <Table
            columns={[
              {
                key: "title",
                header: "Title",
                cell: (row) => (
                  <A href={`/posts/${row.id}`} class="link link-primary">
                    {row.title}
                  </A>
                ),
              },
              {
                key: "subreddit",
                header: "Subreddit",
                cell: (row) => (
                  <span
                    class="tooltip cursor-help underline decoration-dotted"
                    data-tip={`Community for ${row.subreddit.replace("r/", "")}`}
                  >
                    {row.subreddit}
                  </span>
                ),
              },
              { key: "score", header: "Score", class: "text-right" },
              {
                key: "actions",
                header: "",
                cell: (row) => (
                  <Button variant="ghost" size="sm" href={`/posts/${row.id}`}>
                    details
                  </Button>
                ),
              },
            ]}
            data={posts}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Complex Cells</h2>
          <Table
            columns={[
              {
                key: "title",
                header: "Post",
                cell: (row) => (
                  <Stack gap="sm">
                    <span class="font-medium">{row.title}</span>
                    <span class="badge badge-ghost badge-sm w-fit">
                      {row.subreddit}
                    </span>
                  </Stack>
                ),
              },
              { key: "score", header: "Score", class: "text-right" },
              {
                key: "date",
                header: "Date",
              },
              {
                key: "actions",
                header: "",
                cell: (row) => (
                  <Button variant="ghost" size="sm" href={`/posts/${row.id}`}>
                    details
                  </Button>
                ),
              },
            ]}
            data={posts}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Custom Cells</h2>
          <Table columns={actionColumns} data={posts} />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Load States</h2>
          <p class="text-base-content/60 text-sm">
            Single loadState prop: normal | pending | fetching
          </p>
          <Stack gap="md">
            <Row class="flex-wrap" gap="sm">
              <Button variant="ghost" onClick={() => setLoadState("normal")}>
                Normal
              </Button>
              <Button variant="primary" onClick={() => setLoadState("pending")}>
                Pending (no data)
              </Button>
              <Button
                variant="primary"
                onClick={() => setLoadState("fetching")}
              >
                Fetching (has stale)
              </Button>
            </Row>
            <Table
              columns={baseColumns}
              data={loadState() === "pending" ? [] : posts}
              loadState={loadState()}
            />
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Empty State</h2>
          <Stack gap="md">
            <Row gap="sm">
              <Button variant="primary" onClick={() => setEmpty((v) => !v)}>
                Toggle Empty
              </Button>
            </Row>
            <Table
              columns={baseColumns}
              data={data()}
              emptyMessage="No posts match your filters"
            />
          </Stack>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
