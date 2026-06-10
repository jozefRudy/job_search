import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import { Container } from "~/components/ui/layout/Container";
import { Grid } from "~/components/ui/layout/Grid";
import type { Size } from "~/components/ui/layout/layout";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "../DevLayout";

const sizes: Size[] = ["sm", "md", "lg"];

export default function CardPage() {
  return (
    <DevLayout title="Card Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Variants</h2>
          <Grid cols={1} mdCols={2} gap="md">
            <Card>
              <Card.Body>
                <Card.Title>Default</Card.Title>
                <p class="text-base-content/70">Card without border.</p>
              </Card.Body>
            </Card>
            <Card class="card-border border-primary">
              <Card.Body>
                <Card.Title>Bordered</Card.Title>
                <p class="text-base-content/70">Card with border via class.</p>
              </Card.Body>
            </Card>
          </Grid>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Hoverable</h2>
          <Grid cols={1} mdCols={2} gap="md">
            <Card behavior="hoverable">
              <Card.Body>
                <Card.Title>Default Hover</Card.Title>
                <p class="text-base-content/70">Hover over this card.</p>
              </Card.Body>
            </Card>
            <Card class="card-border border-primary" behavior="hoverable">
              <Card.Body>
                <Card.Title>Bordered Hover</Card.Title>
                <p class="text-base-content/70">Hover over this card.</p>
              </Card.Body>
            </Card>
          </Grid>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">With Actions</h2>
          <Grid cols={1} mdCols={2} gap="md">
            <Card>
              <Card.Body>
                <Card.Title>Default Actions</Card.Title>
                <p class="text-base-content/70">Card with action buttons.</p>
                <Card.Actions class="justify-end">
                  <Button variant="ghost" size="sm">
                    Cancel
                  </Button>
                  <Button variant="primary" size="sm">
                    Save
                  </Button>
                </Card.Actions>
              </Card.Body>
            </Card>
            <Card class="card-border border-primary">
              <Card.Body>
                <Card.Title>Bordered Actions</Card.Title>
                <p class="text-base-content/70">Card with action buttons.</p>
                <Card.Actions class="justify-end">
                  <Button variant="ghost" size="sm">
                    Cancel
                  </Button>
                  <Button variant="primary" size="sm">
                    Save
                  </Button>
                </Card.Actions>
              </Card.Body>
            </Card>
          </Grid>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Sizes</h2>
          <Grid cols={1} mdCols={3} gap="md">
            {sizes.map((size) => (
              <Card size={size} class="card-border border-primary">
                <Card.Body>
                  <Card.Title class="capitalize">{size}</Card.Title>
                  <p class="text-base-content/70">size="{size}"</p>
                  <Card.Actions>
                    <Button variant="primary" size="sm">
                      Action
                    </Button>
                  </Card.Actions>
                </Card.Body>
              </Card>
            ))}
          </Grid>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Clickable Cards</h2>
          <Grid cols={1} mdCols={3} gap="md">
            {[
              { title: "Dashboard", desc: "View analytics", icon: "filter" },
              { title: "Settings", desc: "Configure app", icon: "tick" },
              { title: "Profile", desc: "Edit account", icon: "arrow" },
            ].map((item) => (
              <Card behavior="clickable" class="group">
                <Card.Body>
                  <Row align="start" justify="between">
                    <Card.Title class="transition-colors group-hover:text-primary">
                      {item.title}
                    </Card.Title>
                    <svg
                      class="h-5 w-5 text-base-content/40 transition-colors group-hover:text-primary"
                      fill="currentColor"
                      aria-label={item.icon}
                    >
                      <use href={`/assets/icons.svg#icon-${item.icon}`} />
                    </svg>
                  </Row>
                  <p class="text-base-content/70">{item.desc}</p>
                </Card.Body>
              </Card>
            ))}
          </Grid>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Pricing Card Pattern</h2>
          <Container maxWidth="sm">
            <Card class="card-border border-primary">
              <Card.Body>
                <Row align="center" justify="between">
                  <span class="badge badge-accent">Most Popular</span>
                  <span class="font-bold text-2xl">$29/mo</span>
                </Row>
                <Card.Title>Premium</Card.Title>
                <ul class="space-y-2 text-base-content/70 text-sm">
                  <li>
                    <Row align="center" gap="sm">
                      <svg
                        class="h-4 w-4 text-success"
                        fill="currentColor"
                        aria-label="Included"
                      >
                        <use href="/assets/icons.svg#icon-tick" />
                      </svg>
                      High-resolution image generation
                    </Row>
                  </li>
                  <li>
                    <Row align="center" gap="sm">
                      <svg
                        class="h-4 w-4 text-success"
                        fill="currentColor"
                        aria-label="Included"
                      >
                        <use href="/assets/icons.svg#icon-tick" />
                      </svg>
                      Customizable style templates
                    </Row>
                  </li>
                  <li>
                    <Row align="center" gap="sm">
                      <svg
                        class="h-4 w-4 text-success"
                        fill="currentColor"
                        aria-label="Included"
                      >
                        <use href="/assets/icons.svg#icon-tick" />
                      </svg>
                      Batch processing capabilities
                    </Row>
                  </li>
                </ul>
                <Card.Actions>
                  <Button variant="primary" class="w-full">
                    Subscribe
                  </Button>
                </Card.Actions>
              </Card.Body>
            </Card>
          </Container>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
