import { A } from "@solidjs/router";
import { Card } from "~/components/ui/Card";
import { Grid } from "~/components/ui/layout/Grid";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "./DevLayout";

const designPages = [
  {
    path: "/dev/buttons",
    label: "Buttons",
    desc: "All variants, sizes, states, icons",
  },
  {
    path: "/dev/cards",
    label: "Cards",
    desc: "Variants, sizes, hoverable, actions, pricing",
  },
  {
    path: "/dev/tables",
    label: "Tables",
    desc: "Sizes, zebra, custom cells, loading, empty",
  },
  {
    path: "/dev/pagination",
    label: "Pagination",
    desc: "Few pages, many pages, edge cases",
  },
  {
    path: "/dev/inputs",
    label: "Inputs",
    desc: "FormField, PasswordInput, Range",
  },
  {
    path: "/dev/layout",
    label: "Layout",
    desc: "Stack, Row, Grid, Container, Section",
  },
  {
    path: "/dev/skeleton",
    label: "Skeleton",
    desc: "Block, circle, text shimmer, card layouts",
  },
  {
    path: "/dev/sidebar",
    label: "Sidebar",
    desc: "Responsive sidebar layout with drawer",
  },
  {
    path: "/dev/modals",
    label: "Modals",
    desc: "Modal, ConfirmModal, sizes, variants",
  },
];

const appPages = [
  {
    path: "/dev/app/job-detail",
    label: "JobDetail",
    desc: "Job detail view for Upwork and NoFluffJobs",
  },
];

function PageGrid(props: { pages: typeof designPages }) {
  return (
    <Grid cols={1} mdCols={2} lgCols={3} gap="md">
      {props.pages.map((p) => (
        <A href={p.path} class="contents">
          <Card class="card-border border-primary" behavior="clickable">
            <Card.Body>
              <Card.Title>{p.label}</Card.Title>
              <p class="text-base-content/60">{p.desc}</p>
            </Card.Body>
          </Card>
        </A>
      ))}
    </Grid>
  );
}

export default function DevIndex() {
  return (
    <DevLayout title="Dev Components">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Design System</h2>
          <PageGrid pages={designPages} />
        </Stack>
        <Stack gap="md">
          <h2 class="font-semibold text-xl">App Components</h2>
          <PageGrid pages={appPages} />
        </Stack>
      </Stack>
    </DevLayout>
  );
}
