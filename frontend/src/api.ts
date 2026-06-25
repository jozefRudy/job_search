import { useNavigate } from "@solidjs/router";
import { createQuery, useQueryClient } from "@tanstack/solid-query";
import {
  getGetJobQueryKey,
  getJob,
  getListJobsQueryKey,
  listJobs,
  useApplyJob as useApplyJobGen,
  useDeleteJob as useDeleteJobGen,
  useRateJob as useRateJobGen,
} from "~/generated/orval/jobsearch";
import type { ListJobsParams } from "~/generated/orval/jobsearch.schemas";

export type {
  ApplyRequest,
  Job,
  ListJobsParams,
  RateRequest,
  Sort,
} from "~/generated/orval/jobsearch.schemas";
export {
  Platform,
  Rating,
} from "~/generated/orval/jobsearch.schemas";

export function useListJobs(params: () => ListJobsParams) {
  return createQuery(() => ({
    queryKey: getListJobsQueryKey(params()),
    queryFn: () => listJobs(params()).then((res) => res.data),
    retry: false,
    structuralSharing: false,
  }));
}

export function useGetJob(id: () => number) {
  return createQuery(() => ({
    queryKey: getGetJobQueryKey(id()),
    queryFn: () => getJob(id()).then((res) => res.data),
    enabled: id() != null && !Number.isNaN(id()),
    retry: false,
    structuralSharing: false,
  }));
}

export function useRateJob() {
  const qc = useQueryClient();
  return useRateJobGen({
    mutation: {
      onSuccess: (_data, variables) => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        qc.invalidateQueries({ queryKey: getGetJobQueryKey(variables.id) });
      },
    },
  });
}

export function useApplyJob() {
  const qc = useQueryClient();
  return useApplyJobGen({
    mutation: {
      onSuccess: (_data, variables) => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        qc.invalidateQueries({ queryKey: getGetJobQueryKey(variables.id) });
      },
    },
  });
}

export function useDeleteJob() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  return useDeleteJobGen({
    mutation: {
      onSuccess: () => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        navigate(-1);
      },
    },
  });
}
