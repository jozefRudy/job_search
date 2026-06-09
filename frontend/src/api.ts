import type {
  Job,
  JobListResponse,
  ListQuery,
  Platform,
  Rating,
  Sort,
} from "./generated";

export type {
  Data,
  Job,
  JobListResponse,
  ListQuery,
  Platform,
  Rating,
  Sort,
} from "./generated";

export async function listJobs(query: ListQuery): Promise<JobListResponse> {
  const params = new URLSearchParams();
  if (query.platform != null) params.set("platform", query.platform);
  if (query.rating != null) params.set("rating", query.rating);
  params.set("sort_by", query.sort_by);
  params.set("page", String(query.page));
  params.set("page_size", String(query.page_size));
  const res = await fetch(`/api/jobs?${params}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function getJob(id: number): Promise<Job> {
  const res = await fetch(`/api/jobs/${id}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function rateJob(id: number, rating: Rating): Promise<void> {
  const res = await fetch(`/api/jobs/${id}/rate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ rating }),
  });
  if (!res.ok) throw new Error(await res.text());
}
