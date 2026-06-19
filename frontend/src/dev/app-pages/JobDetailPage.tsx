import { JobDetailContent } from "~/components/JobDetail";
import { Stack } from "~/components/ui/layout/Stack";
import type { Job } from "~/generated/orval/jobsearch.schemas";
import { DevLayout } from "../DevLayout";

const mockUpworkJob: Job = {
  id: 1,
  external_id: "upwork-123",
  platform: "upwork",
  title: "Full-Stack Rust Developer for Web3 Project",
  url: "https://www.upwork.com/jobs/~0123456789",
  budget: "$5,000 – $10,000",
  description: "Looking for an experienced Rust developer...",
  tags: ["rust", "web3", "solidity", "react"],
  created_at: "2026-06-08T10:00:00Z",
  updated_at: "2026-06-09T14:00:00Z",
  liked: true,
  applied_at: null,
  note: null,
  raw: {
    platform: "upwork",
    detail: {
      description:
        "We are building a decentralized exchange and need a senior Rust engineer to lead the backend development.\n\nResponsibilities:\n- Design and implement core trading engine\n- Optimize for low latency and high throughput\n- Write comprehensive tests",
      exact_budget: "$7,500",
      experience_level: "Expert",
      project_type: "Complex project",
      duration: "3 to 6 months",
      hours_per_week: "30+ hrs/week",
      hires: "1",
      proposals: "15 to 20",
      last_viewed: "2026-06-09T12:00:00Z",
      posted_at: "2026-06-05T08:00:00Z",
      interviewing: "2",
      invites_sent: "5",
      unanswered_invites: "1",
      tags: ["rust", "web3", "solidity", "react"],
    },
  },
};

const mockNoFluffJob: Job = {
  id: 2,
  external_id: "nfj-456",
  platform: "nofluffjobs",
  title: "Senior Backend Engineer",
  url: "https://nofluffjobs.com/job/senior-backend-engineer",
  budget: "30 000 – 45 000 PLN",
  description: "Join our platform team...",
  tags: ["rust", "postgres", "kafka", "docker"],
  created_at: "2026-06-05T08:00:00Z",
  updated_at: "2026-06-09T10:00:00Z",
  liked: null,
  applied_at: "2026-06-07T09:00:00Z",
  note: "Applied via referral. Waiting for response.",
  raw: {
    platform: "nofluffjobs",
    detail: {
      company: "TechCorp Poland",
      seniority: "Senior",
      locations: ["Warsaw", "Kraków"],
      offer_valid_until: "2026-07-01",
      posted_at: "2026-06-05T08:00:00Z",
      languages: ["English B2", "Polish native"],
      must_have: ["Rust", "PostgreSQL", "5+ years experience"],
      requirements:
        "- 5+ years of backend development\n- Strong Rust skills\n- Experience with distributed systems",
      nice_to_have:
        "- Kafka experience\n- Kubernetes\n- Open source contributions",
      description:
        "We are looking for a Senior Backend Engineer to join our core platform team. You will design and build scalable services handling millions of requests per day.",
    },
  },
};

export default function JobDetailPage() {
  return (
    <DevLayout title="JobDetail" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Upwork — Liked</h2>
          <JobDetailContent job={mockUpworkJob} />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">NoFluffJobs — Applied</h2>
          <JobDetailContent job={mockNoFluffJob} />
        </Stack>
      </Stack>
    </DevLayout>
  );
}
