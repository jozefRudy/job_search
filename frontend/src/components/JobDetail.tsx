import { useNavigate, useParams } from "@solidjs/router";
import { createResource, For, Show } from "solid-js";
import { getJob, type Job, type Rating, rateJob } from "~/api";
import { Button } from "~/components/ui/Button";
import { Container } from "~/components/ui/layout/Container";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { Skeleton } from "~/components/ui/Skeleton";
import { fmtRelative } from "~/lib/utils";

export function JobDetail() {
  const params = useParams();
  const navigate = useNavigate();
  const id = () => Number(params.id);

  const [job, { refetch }] = createResource(id, (i) => getJob(i));

  async function handleRate(rating: Rating) {
    await rateJob(id(), rating);
    await refetch();
  }

  return (
    <Container maxWidth="md" paddingX="sm" class="py-6">
      <Stack gap="md">
        <Button
          variant="link"
          size="sm"
          class="self-start"
          onClick={() => navigate("/")}
        >
          ← Back
        </Button>

        <Show when={job()} fallback={<Skeleton class="h-96" />}>
          {(j) => <JobDetailContent job={j()} onRate={handleRate} />}
        </Show>
      </Stack>
    </Container>
  );
}

function JobDetailContent(props: { job: Job; onRate: (r: Rating) => void }) {
  const j = props.job;

  return (
    <Stack gap="md">
      <div class="card bg-base-200">
        <div class="card-body">
          <h2 class="card-title text-xl">
            #{j.id} [{j.platform}] {j.title}
          </h2>
          <p class="text-base-content/70">
            {fmtRelative(j.created_at)} | {j.budget ?? "?"}
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

      <Show when={j.platform === "upwork"}>
        <UpworkDetail job={j} />
      </Show>
      <Show when={j.platform === "nofluffjobs"}>
        <NoFluffDetail job={j} />
      </Show>

      <div class="card bg-base-200">
        <div class="card-body">
          <h3 class="card-title text-lg">Actions</h3>
          <p class="text-base-content/70 text-sm">
            Current:{" "}
            {j.liked === true
              ? "👍 Liked"
              : j.liked === false
                ? "👎 Disliked"
                : "↔️ Neutral"}
          </p>
          <Row gap="sm">
            <Button
              variant={j.liked === true ? "primary" : "ghost"}
              size="sm"
              onClick={() => props.onRate("liked")}
            >
              👍 Like
            </Button>
            <Button
              variant={j.liked === false ? "danger" : "ghost"}
              size="sm"
              onClick={() => props.onRate("disliked")}
            >
              👎 Dislike
            </Button>
            <Button
              variant={j.liked === null ? "secondary" : "ghost"}
              size="sm"
              onClick={() => props.onRate("neutral")}
            >
              ↔️ Neutral
            </Button>
          </Row>
          <Show when={j.applied_at}>
            <p class="text-success">Applied {fmtRelative(j.applied_at)}</p>
          </Show>
          <Show when={j.note}>
            <p class="whitespace-pre-line text-base-content/70">{j.note}</p>
          </Show>
        </div>
      </div>
    </Stack>
  );
}

function UpworkDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "upwork") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
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
          <Show when={d.description}>
            <div>
              <span class="font-semibold">Description:</span>
              <p class="mt-1 whitespace-pre-line">{d.description}</p>
            </div>
          </Show>
        </Stack>
      </div>
    </div>
  );
}

function NoFluffDetail(props: { job: Job }) {
  const raw = props.job.raw;
  if (raw.platform !== "nofluffjobs") return null;
  const d = raw.detail;
  return (
    <div class="card bg-base-200">
      <div class="card-body">
        <h3 class="card-title text-lg">Details</h3>
        <Stack gap="sm">
          <DetailRow label="Company" value={d.company} />
          <DetailRow label="Seniority" value={d.seniority} />
          <DetailRow label="Remote" value={d.remote} />
          <Show when={d.locations.length > 0}>
            <DetailRow label="Locations" value={d.locations.join(", ")} />
          </Show>
          <DetailRow label="Valid until" value={d.offer_valid_until} />
          <Show when={d.must_have.length > 0}>
            <DetailRow label="Must have" value={d.must_have.join(", ")} />
          </Show>
          <Show when={d.languages.length > 0}>
            <DetailRow label="Languages" value={d.languages.join(", ")} />
          </Show>
          <Show when={d.requirements}>
            <div>
              <span class="font-semibold">Requirements:</span>
              <p class="mt-1 whitespace-pre-line">{d.requirements}</p>
            </div>
          </Show>
          <Show when={d.nice_to_have}>
            <div>
              <span class="font-semibold">Nice to have:</span>
              <p class="mt-1 whitespace-pre-line">{d.nice_to_have}</p>
            </div>
          </Show>
          <Show when={d.description}>
            <div>
              <span class="font-semibold">Description:</span>
              <p class="mt-1 whitespace-pre-line">{d.description}</p>
            </div>
          </Show>
        </Stack>
      </div>
    </div>
  );
}

function DetailRow(props: { label: string; value: string }) {
  if (!props.value) return null;
  return (
    <div class="grid grid-cols-3 gap-2">
      <span class="font-semibold text-base-content/70">{props.label}</span>
      <span class="col-span-2">{props.value}</span>
    </div>
  );
}
