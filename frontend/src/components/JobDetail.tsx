import { useNavigate, useParams } from "@solidjs/router";
import { createSignal, For, type JSX, Show } from "solid-js";
import {
  type Job,
  type Rating,
  useApplyJob,
  useDeleteJob,
  useGetJob,
  useRateJob,
} from "~/api";
import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import { ErrorAlert } from "~/components/ui/ErrorAlert";
import { Icon } from "~/components/ui/Icon";
import { Container } from "~/components/ui/layout/Container";
import { Grid } from "~/components/ui/layout/Grid";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { Markdown } from "~/components/ui/Markdown";
import { ConfirmModal } from "~/components/ui/Modal";
import { Skeleton } from "~/components/ui/Skeleton";
import { Swap } from "~/components/ui/Swap";
import { fmtRelative } from "~/lib/utils";

export function JobDetail() {
  const params = useParams();
  const navigate = useNavigate();
  const id = () => Number(params.id);

  const jobQuery = useGetJob(id);

  return (
    <Container maxWidth="md" paddingX="sm" class="py-6">
      <Stack gap="md">
        <Button
          variant="link"
          size="sm"
          class="self-start"
          onClick={() => navigate(-1)}
        >
          ← Back
        </Button>

        <Show when={jobQuery.error} keyed>
          {(err) => <ErrorAlert>Error loading job: {err.message}</ErrorAlert>}
        </Show>
        <Show when={!jobQuery.error}>
          <Show when={jobQuery.data} keyed fallback={<Skeleton class="h-96" />}>
            {(j) => <JobDetailContent job={j} />}
          </Show>
        </Show>
      </Stack>
    </Container>
  );
}

export function JobDetailContent(props: { job: Job }) {
  const j = props.job;
  const rateMutation = useRateJob();
  const applyMutation = useApplyJob();
  const deleteMutation = useDeleteJob();
  const [showDelete, setShowDelete] = createSignal(false);

  function handleRate(rating: Rating) {
    rateMutation.mutate({ id: j.id, data: { rating } });
  }

  function handleApply(applied: boolean) {
    applyMutation.mutate({ id: j.id, data: { applied } });
  }

  function handleDelete() {
    deleteMutation.mutate({ id: j.id });
  }

  const mutationError = () =>
    applyMutation.error?.message ??
    deleteMutation.error?.message ??
    rateMutation.error?.message;

  return (
    <Stack gap="md">
      <Show when={mutationError()}>
        {(msg) => <ErrorAlert>Error updating job: {msg()}</ErrorAlert>}
      </Show>

      <div class="card bg-base-200">
        <div class="card-body">
          <h2 class="card-title text-xl">
            #{j.id} [{j.platform}] {j.title}
          </h2>
          <p class="text-base-content/70">
            {fmtRelative(j.created_at)} | {j.budget ?? "no budget available"}
          </p>
          <p>
            <a
              href={j.url}
              target="_blank"
              class="link link-primary"
              rel="noopener"
            >
              {j.url}
            </a>
          </p>
          <Show when={j.tags.length > 0}>
            <Row gap="sm" class="flex-wrap">
              <For each={j.tags}>
                {(tag) => <span class="badge badge-sm">{tag}</span>}
              </For>
            </Row>
          </Show>
        </div>
      </div>

      <Card>
        <Card.Body>
          <Card.Title class="text-lg">Actions</Card.Title>
          <Row gap="sm" align="center">
            <Button
              variant={j.liked === true ? "primary" : "ghost"}
              size="sm"
              onClick={() => handleRate("liked")}
            >
              👍 Like
            </Button>
            <Button
              variant={j.liked === false ? "danger" : "ghost"}
              size="sm"
              onClick={() => handleRate("disliked")}
            >
              👎 Dislike
            </Button>
            <Button
              variant={j.liked === null ? "secondary" : "ghost"}
              size="sm"
              onClick={() => handleRate("neutral")}
            >
              ↔️ Neutral
            </Button>
            <div class="divider divider-horizontal mx-1" />
            <Swap
              checked={!!j.applied_at}
              onChange={(applied) => handleApply(applied)}
              size="sm"
              on={
                <Row gap="sm" align="center">
                  <Icon
                    id="cancel_schedule_send"
                    class="size-[1em] text-error"
                  />
                  <span class="text-xs">Un-apply</span>
                </Row>
              }
              off={
                <Row gap="sm" align="center">
                  <Icon id="send" class="size-[1em] text-primary" />
                  <span class="text-xs">Apply</span>
                </Row>
              }
            />
          </Row>
          <div>
            <Button
              variant="danger"
              size="sm"
              onClick={() => setShowDelete(true)}
            >
              🗑️ Delete
            </Button>
          </div>
        </Card.Body>
      </Card>

      <Show when={j.platform === "upwork"}>
        <UpworkDetail job={j} />
      </Show>
      <Show when={j.platform === "nofluffjobs"}>
        <NoFluffDetail job={j} />
      </Show>
      <Show when={j.platform === "efinancialcareers"}>
        <EfinancialcareersDetail job={j} />
      </Show>
      <Show when={j.platform === "hackernews"}>
        <HackerNewsDetail job={j} />
      </Show>

      <ApplicationCard appliedAt={j.applied_at} note={j.note} />

      <ConfirmModal
        open={showDelete()}
        onClose={() => setShowDelete(false)}
        onConfirm={() => {
          setShowDelete(false);
          handleDelete();
        }}
        title="Delete job?"
        message={`Delete "${props.job.title}"? This cannot be undone.`}
        confirmText="Delete"
        confirmVariant="danger"
      />
    </Stack>
  );
}

export function UpworkDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "upwork") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
          <DetailList>
            <DetailRow label="Exact budget" value={d.exact_budget} />
            <DetailRow label="Experience" value={d.experience_level} />
            <DetailRow label="Project type" value={d.project_type} />
            <DetailRow label="Duration" value={d.duration} />
            <DetailRow label="Hours/week" value={d.hours_per_week} />
            <DetailRow label="Hires" value={d.hires} />
            <DetailRow label="Proposals" value={d.proposals} />
            <DetailRow
              label="Last viewed"
              value={d.last_viewed ? fmtRelative(d.last_viewed) : "never"}
            />
            <DetailRow label="Interviewing" value={d.interviewing} />
            <DetailRow label="Invites sent" value={d.invites_sent} />
            <DetailRow label="Unanswered" value={d.unanswered_invites} />
          </DetailList>
          <Show when={d.description}>
            <MarkdownDescription label="Description" text={d.description} />
          </Show>
        </Stack>
      </div>
    </div>
  );
}

export function NoFluffDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "nofluffjobs") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
          <DetailList>
            <DetailRow label="Company" value={d.company} />
            <DetailRow label="Seniority" value={d.seniority} />
            <Show when={d.locations && d.locations.length > 0}>
              <DetailRow
                label="Locations"
                value={d.locations?.join(", ") ?? ""}
              />
            </Show>
            <DetailRow label="Valid until" value={d.offer_valid_until} />
            <Show when={d.must_have && d.must_have.length > 0}>
              <DetailRow
                label="Must have"
                value={d.must_have?.join(", ") ?? ""}
              />
            </Show>
            <Show when={d.languages && d.languages.length > 0}>
              <DetailRow
                label="Languages"
                value={d.languages?.join(", ") ?? ""}
              />
            </Show>
          </DetailList>
          <Show when={d.requirements}>
            <MarkdownDescription label="Requirements" text={d.requirements} />
          </Show>
          <Show when={d.nice_to_have}>
            <MarkdownDescription label="Nice to have" text={d.nice_to_have} />
          </Show>
          <Show when={d.description}>
            <MarkdownDescription label="Description" text={d.description} />
          </Show>
        </Stack>
      </div>
    </div>
  );
}

export function HackerNewsDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "hackernews") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
          <DetailList>
            <DetailRow label="Author" value={d.author} />
            <DetailRow label="Company" value={d.company ?? props.job.company} />
            <DetailRow label="Role" value={d.role} />
            <DetailRow label="Location" value={d.location} />
            <DetailRow label="Remote" value={d.remote ? "yes" : "no"} />
          </DetailList>
          <Show when={props.job.description} keyed>
            {(text) => <MarkdownDescription label="Description" text={text} />}
          </Show>
        </Stack>
      </div>
    </div>
  );
}

export function EfinancialcareersDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "efinancialcareers") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
          <DetailList>
            <DetailRow label="Company" value={d.company} />
            <DetailRow label="Location" value={d.location} />
            <DetailRow label="Remote" value={d.remote ? "yes" : "no"} />
            <DetailRow label="Employment type" value={d.employment_type} />
            <DetailRow label="Posted" value={fmtRelative(d.posted_at)} />
          </DetailList>
          <Show when={d.description}>
            <MarkdownDescription label="Description" text={d.description} />
          </Show>
        </Stack>
      </div>
    </div>
  );
}

function MarkdownDescription(props: { label: string; text: string }) {
  return (
    <div>
      <span class="font-semibold">{props.label}:</span>
      <Markdown class="mt-1" text={props.text} />
    </div>
  );
}

function ApplicationCard(props: {
  appliedAt?: string | null;
  note?: string | null;
}) {
  return (
    <Card>
      <Card.Body>
        <Card.Title class="text-lg">Job Application</Card.Title>
        <Show
          when={props.appliedAt}
          fallback={<p class="text-base-content/70">Not applied yet.</p>}
        >
          {(d) => <p class="text-success">Applied {fmtRelative(d())}</p>}
        </Show>
        <Show when={props.note}>
          {(n) => <Markdown class="text-base-content/70" text={n()} />}
        </Show>
      </Card.Body>
    </Card>
  );
}

function DetailList(props: { children: JSX.Element }) {
  return (
    <Grid cols={2} gap="sm" class="grid-cols-[max-content_1fr]">
      {props.children}
    </Grid>
  );
}

function DetailRow(props: { label: string; value: string | null | undefined }) {
  if (!props.value) return null;
  return (
    <>
      <span class="font-semibold text-base-content/70">{props.label}</span>
      <span>{props.value}</span>
    </>
  );
}
