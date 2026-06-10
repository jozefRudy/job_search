import type { ButtonState, ButtonVariant } from "~/components/ui/Button";
import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import type { Size } from "~/components/ui/layout/layout";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "../DevLayout";

const variants: ButtonVariant[] = [
  "primary",
  "secondary",
  "ghost",
  "danger",
  "link",
];
const sizes: Size[] = ["sm", "md", "lg"];
const states: ButtonState[] = ["normal", "disabled", "loading"];

export default function ButtonPage() {
  return (
    <DevLayout title="Button Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">All Variants × Sizes × States</h2>
          <Stack gap="lg">
            {variants.map((variant) => (
              <Card>
                <Card.Body>
                  <Stack gap="md">
                    <h3 class="font-medium text-lg capitalize">{variant}</h3>
                    <Stack gap="md">
                      {sizes.map((size) => (
                        <Row align="center" gap="md">
                          <span class="w-12 text-sm">{size}</span>
                          {states.map((state) => (
                            <Button variant={variant} size={size} state={state}>
                              {state}
                            </Button>
                          ))}
                        </Row>
                      ))}
                    </Stack>
                  </Stack>
                </Card.Body>
              </Card>
            ))}
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">With Icons</h2>
          <Card>
            <Card.Body>
              <Stack gap="lg">
                <Stack gap="sm">
                  <h3 class="font-medium text-base-content/60 text-sm">
                    variant (with icon)
                  </h3>
                  <Row align="center" gap="md">
                    <span class="w-12 text-sm">md</span>
                    <Button variant="link" icon="arrow" iconPlacement="right">
                      Link
                    </Button>
                    <Button
                      variant="link"
                      state="loading"
                      icon="arrow"
                      iconPlacement="right"
                    >
                      Loading
                    </Button>
                  </Row>
                </Stack>
                <Stack gap="sm">
                  <h3 class="font-medium text-base-content/60 text-sm">
                    iconPlacement
                  </h3>
                  <Row align="center" gap="md">
                    <span class="w-12 text-sm">md</span>
                    <Button
                      variant="secondary"
                      icon="filter"
                      iconPlacement="left"
                    >
                      Left
                    </Button>
                    <Button
                      variant="primary"
                      icon="arrow"
                      iconPlacement="right"
                    >
                      Right
                    </Button>
                    <Button
                      variant="primary"
                      state="loading"
                      icon="arrow"
                      iconPlacement="right"
                    >
                      Loading
                    </Button>
                  </Row>
                </Stack>
                <Stack gap="sm">
                  <h3 class="font-medium text-sm">size</h3>
                  <Stack gap="md">
                    {sizes.map((size) => (
                      <Row align="center" gap="md">
                        <span class="w-12 text-base-content/60 text-sm">
                          {size}
                        </span>
                        <Button
                          variant="primary"
                          icon="arrow"
                          iconPlacement="right"
                          size={size}
                        >
                          Icon Right
                        </Button>
                        <Button
                          variant="danger"
                          icon="close"
                          iconPlacement="right"
                          size={size}
                        >
                          Icon Right
                        </Button>
                      </Row>
                    ))}
                  </Stack>
                </Stack>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Icon Only (no children)</h2>
          <Card>
            <Card.Body>
              <Stack gap="lg">
                {(["ghost", "primary"] as ButtonVariant[]).map((variant) => (
                  <Row align="center" gap="md">
                    <span class="w-16 text-sm capitalize">{variant}</span>
                    {sizes.map((size) => (
                      <Stack align="center" gap="sm">
                        <Button variant={variant} icon="tick" size={size} />
                        <span class="text-xs">{size}</span>
                      </Stack>
                    ))}
                  </Row>
                ))}
              </Stack>
            </Card.Body>
          </Card>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
