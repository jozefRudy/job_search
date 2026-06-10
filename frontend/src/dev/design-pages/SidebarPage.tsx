import { A } from "@solidjs/router";
import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import { Row } from "~/components/ui/layout/Row";
import { SidebarLayout } from "~/components/ui/layout/SidebarLayout";
import { Stack } from "~/components/ui/layout/Stack";
import { ThemeToggle } from "~/components/ui/ThemeToggle";

function Nav() {
  const links = [
    { label: "Dashboard", href: "#" },
    { label: "Posts", href: "#" },
    { label: "Filters", href: "#" },
    { label: "Settings", href: "#" },
  ];
  return (
    <Stack
      gap="sm"
      class="min-h-full w-72 border-base-300 border-r bg-base-200 p-4 text-base-content"
    >
      <h2 class="px-4 font-semibold text-lg">App</h2>
      <ul class="menu w-full">
        {links.map((l) => (
          <li>
            <A href={l.href}>{l.label}</A>
          </li>
        ))}
      </ul>
    </Stack>
  );
}

export default function SidebarPage() {
  return (
    <SidebarLayout sidebar={<Nav />}>
      <div class="p-6">
        <Stack gap="lg">
          <Row align="center" justify="between">
            <Row align="center" gap="md">
              <Button
                variant="link"
                href="/dev"
                icon="back-arrow"
                iconPlacement="left"
                size="sm"
              >
                Back to Dev
              </Button>
              <h1 class="font-bold text-2xl">Sidebar Layout</h1>
            </Row>
            <ThemeToggle />
          </Row>
          <p class="text-base-content/60">
            Resize browser to see drawer toggle on mobile. Sidebar is permanent
            on desktop.
          </p>
          <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
            <Card>
              <Card.Body>
                <Card.Title>Users</Card.Title>
                <p class="font-bold text-3xl">1,234</p>
              </Card.Body>
            </Card>
            <Card>
              <Card.Body>
                <Card.Title>Posts</Card.Title>
                <p class="font-bold text-3xl">567</p>
              </Card.Body>
            </Card>
            <Card>
              <Card.Body>
                <Card.Title>Alerts</Card.Title>
                <p class="font-bold text-3xl">89</p>
              </Card.Body>
            </Card>
          </div>
        </Stack>
      </div>
    </SidebarLayout>
  );
}
