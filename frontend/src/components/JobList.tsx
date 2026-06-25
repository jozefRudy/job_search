import { useNavigate, useSearchParams } from "@solidjs/router";
import { isNotNil, pickBy } from "es-toolkit";
import { createMemo, Show } from "solid-js";
import { z } from "zod";
import {
  type Job,
  type ListJobsParams,
  type Platform,
  Platform as PlatformEnum,
  type Rating,
  Rating as RatingEnum,
  type Sort,
  useListJobs,
  useRateJob,
} from "~/api";
import { Button } from "~/components/ui/Button";
import { Pagination } from "~/components/ui/data/Pagination";
import { Table, type TableLoadState } from "~/components/ui/data/Table";
import { ErrorAlert } from "~/components/ui/ErrorAlert";
import { Container } from "~/components/ui/layout/Container";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { cn, ellip, fmtRelative, ratingClass, ratingEmoji } from "~/lib/utils";

const PAGE_SIZE = 20;

type BoolFilter = "true" | "false" | "any";

const PLATFORM_SORTS: Record<
  Platform | "any",
  ReadonlyArray<{ value: Sort; label: string }>
> = {
  any: [
    { value: "created", label: "Created" },
    { value: "applied", label: "Applied" },
  ],
  upwork: [
    { value: "created", label: "Created" },
    { value: "upwork_viewed", label: "Last viewed" },
    { value: "applied", label: "Applied" },
  ],
  nofluffjobs: [
    { value: "created", label: "Created" },
    { value: "applied", label: "Applied" },
  ],
  efinancialcareers: [
    { value: "created", label: "Created" },
    { value: "applied", label: "Applied" },
  ],
  hackernews: [
    { value: "created", label: "Created" },
    { value: "applied", label: "Applied" },
  ],
};

type WithAny<T extends string> = T | "any";

const PLATFORMS: ReadonlyArray<WithAny<Platform>> = [
  "any",
  ...Object.values(PlatformEnum),
];

const RATINGS: ReadonlyArray<WithAny<Rating>> = [
  "any",
  ...Object.values(RatingEnum),
];

const BOOL_FILTERS: ReadonlyArray<BoolFilter> = ["any", "true", "false"];

const enumSchema = <T extends string>(
  values: ReadonlyArray<WithAny<T>>,
  fallback: WithAny<T>,
) =>
  z
    .union([...values.map((v) => z.literal(v)), z.literal("any")])
    .catch(fallback);

export function JobList() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const platform = (): Platform | "any" =>
    enumSchema(PLATFORMS, "any").parse(searchParams.platform);

  const rating = (): Rating | "any" =>
    enumSchema(RATINGS, "any").parse(searchParams.rating);

  const applied = (): BoolFilter =>
    enumSchema(BOOL_FILTERS, "any").parse(searchParams.applied);
  const remote = (): BoolFilter =>
    enumSchema(BOOL_FILTERS, "any").parse(searchParams.remote);
  const english = (): BoolFilter =>
    enumSchema(BOOL_FILTERS, "any").parse(searchParams.is_english);

  const sortBy = (): Sort => {
    const key = platform();
    const supported = PLATFORM_SORTS[key].map((s) => s.value);
    return z
      .enum(supported as [Sort, ...Sort[]])
      .catch(supported[0])
      .parse(searchParams.sort_by);
  };

  const page = (): number =>
    z.coerce.number().int().positive().catch(1).parse(searchParams.page);

  const params = (): ListJobsParams =>
    pickBy(
      {
        sort_by: sortBy(),
        page: page(),
        page_size: PAGE_SIZE,
        platform: platform() === "any" ? null : platform(),
        rating: rating() === "any" ? null : rating(),
        applied: applied() === "any" ? null : applied() === "true",
        remote: remote() === "any" ? null : remote() === "true",
        is_english: english() === "any" ? null : english() === "true",
      },
      isNotNil,
    ) as ListJobsParams;

  const query = useListJobs(params);
  const rateMutation = useRateJob();

  const jobs = () => query.data?.jobs ?? [];
  const total = () => query.data?.total ?? 0;

  const hasUpwork = createMemo(() => platform() === "upwork");
  const hasCompany = createMemo(() => platform() !== "upwork");

  const tableLoadState = createMemo((): TableLoadState => {
    if (query.error && jobs().length === 0) return "error";
    if (query.isLoading) return "pending";
    if (query.isFetching) return "fetching";
    return "normal";
  });

  function updateSearch(next: Record<string, string>) {
    setSearchParams({ ...searchParams, ...next, page: "1" }, { replace: true });
  }

  function setPageAndUpdate(p: number) {
    window.scrollTo({ top: 0, behavior: "auto" });
    setSearchParams({ page: String(p) }, { replace: true });
  }

  function handleRate(job: Job, rating: Rating) {
    rateMutation.mutate({ id: job.id, data: { rating } });
  }

  const columns = () => {
    const base = [
      {
        key: "id",
        header: "Id",
        cell: (j: Job) => (
          <button
            type="button"
            class="link link-primary"
            onClick={() => navigate(`/jobs/${j.id}`)}
          >
            {j.id}
          </button>
        ),
      },
      ...(platform() === "any"
        ? [
            {
              key: "platform",
              header: "Platform",
              accessor: (j: Job) => j.platform,
            },
          ]
        : []),
      {
        key: "posted",
        header: "Posted",
        accessor: (j: Job) => fmtRelative(j.created_at),
      },
      {
        key: "rating",
        header: "Rating",
        cell: (j: Job) => (
          <span class={cn(ratingClass(j.liked), "font-bold")}>
            {ratingEmoji(j.liked)}
          </span>
        ),
      },
      {
        key: "actions",
        header: "",
        class: "min-w-[100px]",
        cell: (j: Job) => (
          <Row gap="sm" align="center">
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === true && "text-success")}
              onClick={() => handleRate(j, "liked")}
            >
              👍
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === false && "text-error")}
              onClick={() => handleRate(j, "disliked")}
            >
              👎
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === null && "text-warning")}
              onClick={() => handleRate(j, "neutral")}
            >
              ↔️
            </Button>
          </Row>
        ),
      },
      {
        key: "budget",
        header: "Budget",
        accessor: (j: Job) => ellip(j.budget, 20),
      },
      {
        key: "applied",
        header: "Applied",
        accessor: (j: Job) => fmtRelative(j.applied_at),
      },
      ...(hasUpwork()
        ? [
            {
              key: "last_viewed",
              header: "Last viewed",
              accessor: (j: Job) =>
                j.raw.platform === "upwork"
                  ? fmtRelative(j.raw.detail.last_viewed)
                  : "",
            },
          ]
        : []),
      {
        key: "title",
        header: "Title",
        accessor: (j: Job) => ellip(j.title, 40),
      },
      ...(hasCompany()
        ? [
            {
              key: "company",
              header: "Company",
              accessor: (j: Job) => ellip(j.company, 40),
            },
          ]
        : []),
    ];
    return base;
  };

  return (
    <Container maxWidth="lg" paddingX="sm" class="py-6">
      <Stack gap="md">
        <h1 class="font-bold text-2xl">Jobs</h1>

        <Row gap="md" align="center" class="flex-wrap">
          <select
            class="select select-sm"
            value={platform()}
            onChange={(e) => updateSearch({ platform: e.currentTarget.value })}
          >
            <option value="any">Platforms: any</option>
            <option value="upwork">Upwork</option>
            <option value="nofluffjobs">NoFluffJobs</option>
            <option value="efinancialcareers">eFinancialCareers</option>
            <option value="hackernews">Hacker News</option>
          </select>

          <select
            class="select select-sm"
            value={rating()}
            onChange={(e) => updateSearch({ rating: e.currentTarget.value })}
          >
            <option value="any">Liked: any</option>
            <option value="liked">Liked</option>
            <option value="neutral">Neutral</option>
            <option value="disliked">Disliked</option>
          </select>

          <select
            class="select select-sm"
            value={applied()}
            onChange={(e) => updateSearch({ applied: e.currentTarget.value })}
          >
            <option value="any">Applied: any</option>
            <option value="true">Applied</option>
            <option value="false">Not applied</option>
          </select>

          <select
            class="select select-sm"
            value={remote()}
            onChange={(e) => updateSearch({ remote: e.currentTarget.value })}
          >
            <option value="any">Remote: any</option>
            <option value="true">Remote</option>
            <option value="false">Not remote</option>
          </select>

          <select
            class="select select-sm"
            value={english()}
            onChange={(e) =>
              updateSearch({ is_english: e.currentTarget.value })
            }
          >
            <option value="any">Language: any</option>
            <option value="true">Language: English</option>
            <option value="false">Language: non-English</option>
          </select>

          <Show when={PLATFORM_SORTS[platform()].length > 1}>
            <select
              class="select select-sm"
              value={sortBy()}
              onChange={(e) => updateSearch({ sort_by: e.currentTarget.value })}
            >
              {PLATFORM_SORTS[platform()].map(
                (s: { value: Sort; label: string }) => (
                  <option value={s.value}>{s.label}</option>
                ),
              )}
            </select>
          </Show>
        </Row>

        <Stack gap="md">
          <Show when={query.error && jobs().length > 0}>
            <ErrorAlert>Error loading jobs: {query.error?.message}</ErrorAlert>
          </Show>

          <Table
            columns={columns()}
            data={jobs()}
            zebra
            hoverable
            loadState={tableLoadState()}
            error={
              query.error
                ? `Error loading jobs: ${query.error.message}`
                : undefined
            }
            emptyMessage="No jobs match the current filter"
          />

          <Show when={total() > 0}>
            <Pagination
              currentPage={page()}
              totalItems={total()}
              pageSize={PAGE_SIZE}
              onPageChange={setPageAndUpdate}
            />
          </Show>
        </Stack>
      </Stack>
    </Container>
  );
}
