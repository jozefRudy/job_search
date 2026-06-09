import { Route, Router } from "@solidjs/router";
import type { JSX } from "solid-js";
import { JobDetail } from "~/components/JobDetail";
import { JobList } from "~/components/JobList";
import { Row } from "~/components/ui/layout/Row";
import { ThemeToggle } from "~/components/ui/ThemeToggle";

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
    <Router root={Layout}>
      <Route path="/" component={JobList} />
      <Route path="/jobs/:id" component={JobDetail} />
    </Router>
  );
}
