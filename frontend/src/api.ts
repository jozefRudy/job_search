import { useNavigate } from "@solidjs/router";
import {
  createMutation,
  createQuery,
  useQueryClient,
} from "@tanstack/solid-query";
import {
  deleteJob,
  getGetJobQueryKey,
  getJob,
  getListJobsQueryKey,
  listJobs,
  rateJob,
} from "~/generated/orval/jobsearch";
import type {
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
  return createQuery(() => ({
    queryKey: getListJobsQueryKey(params()),
    queryFn: async () => {
      const res = await listJobs(params());
      if (res.status !== 200) throw new Error("Failed to fetch jobs");
      return res.data;
    },
    structuralSharing: false,
  }));
}

export function useGetJob(id: () => number) {
  return createQuery(() => ({
    queryKey: getGetJobQueryKey(id()),
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
    mutationKey: ["rateJob"],
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

export function useDeleteJob() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  return createMutation(() => ({
    mutationKey: ["deleteJob"],
    mutationFn: async (id: number) => {
      const res = await deleteJob(id);
      if (res.status !== 204) throw new Error("Failed to delete job");
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["jobs"] });
      navigate(-1);
    },
  }));
}
