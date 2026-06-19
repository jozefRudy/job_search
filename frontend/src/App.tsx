import { Route, Router } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { lazy } from "solid-js";
import { JobDetail } from "~/components/JobDetail";
import { JobList } from "~/components/JobList";
import { Row } from "~/components/ui/layout/Row";
import { ThemeToggle } from "~/components/ui/ThemeToggle";

const DevIndex = lazy(() => import("./dev/DevIndex"));
const ButtonPage = lazy(() => import("./dev/design-pages/ButtonPage"));
const CardPage = lazy(() => import("./dev/design-pages/CardPage"));
const TablePage = lazy(() => import("./dev/design-pages/TablePage"));
const PaginationPage = lazy(() => import("./dev/design-pages/PaginationPage"));
const InputsPage = lazy(() => import("./dev/design-pages/InputsPage"));
const LayoutPage = lazy(() => import("./dev/design-pages/LayoutPage"));
const SkeletonPage = lazy(() => import("./dev/design-pages/SkeletonPage"));
const SidebarPage = lazy(() => import("./dev/design-pages/SidebarPage"));
const ModalPage = lazy(() => import("./dev/design-pages/ModalPage"));
const SwapPage = lazy(() => import("./dev/design-pages/SwapPage"));
const ErrorAlertPage = lazy(() => import("./dev/design-pages/ErrorAlertPage"));
const JobDetailPage = lazy(() => import("./dev/app-pages/JobDetailPage"));

const queryClient = new QueryClient();

function Layout(props: { children?: JSX.Element }) {
  return (
    <div class="min-h-screen bg-base-100 text-base-content">
      <header class="navbar bg-base-200 px-4">
        <div class="flex-1">
          <a href="/" class="font-bold text-xl">
            Job Search
          </a>
        </div>
        <Row gap="sm" align="center">
          <ThemeToggle />
        </Row>
      </header>
      <main>{props.children}</main>
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router root={Layout}>
        <Route path="/" component={JobList} />
        <Route path="/jobs/:id" component={JobDetail} />
        {import.meta.env.DEV && (
          <>
            <Route path="/dev" component={DevIndex} />
            <Route path="/dev/buttons" component={ButtonPage} />
            <Route path="/dev/cards" component={CardPage} />
            <Route path="/dev/tables" component={TablePage} />
            <Route path="/dev/pagination" component={PaginationPage} />
            <Route path="/dev/inputs" component={InputsPage} />
            <Route path="/dev/layout" component={LayoutPage} />
            <Route path="/dev/skeleton" component={SkeletonPage} />
            <Route path="/dev/sidebar" component={SidebarPage} />
            <Route path="/dev/modals" component={ModalPage} />
            <Route path="/dev/swap" component={SwapPage} />
            <Route path="/dev/error-alert" component={ErrorAlertPage} />
            <Route path="/dev/app/job-detail" component={JobDetailPage} />
          </>
        )}
      </Router>
    </QueryClientProvider>
  );
}
