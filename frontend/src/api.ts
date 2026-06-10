import {
  createMutation,
  createQuery,
  useQueryClient,
} from "@tanstack/solid-query";
import { getJob, listJobs, rateJob } from "~/generated/orval/jobsearch";
import type {
  Job,
  JobListResponse,
  ListJobsParams,
  Rating,
} from "~/generated/orval/jobsearch.schemas";

export type {
  Data,
  Job,
  JobListResponse,
  ListJobsParams,
  Platform,
  RateBody,
  Rating,
  Sort,
} from "~/generated/orval/jobsearch.schemas";

export function useListJobs(params: () => ListJobsParams) {
  return createQuery<JobListResponse>(() => ({
    queryKey: ["jobs", params()],
    queryFn: async () => {
      const res = await listJobs(params());
      if (res.status !== 200) throw new Error("Failed to fetch jobs");
      return res.data;
    },
    structuralSharing: false,
  }));
}

export function useGetJob(id: () => number) {
  return createQuery<Job>(() => ({
    queryKey: ["job", id()],
    queryFn: async () => {
      const res = await getJob(id());
      if (res.status !== 200) throw new Error("Failed to fetch job");
      return res.data;
    },
    enabled: id() != null && !Number.isNaN(id()),
    structuralSharing: false,
  }));
}

export function useRateJob() {
  const qc = useQueryClient();
  return createMutation(() => ({
    mutationFn: async (vars: { id: number; rating: Rating }) => {
      const res = await rateJob(vars.id, { rating: vars.rating });
      if (res.status !== 204) throw new Error("Failed to rate job");
      return res.data;
    },
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["jobs"] });
      qc.invalidateQueries({ queryKey: ["job", vars.id] });
    },
  }));
}
