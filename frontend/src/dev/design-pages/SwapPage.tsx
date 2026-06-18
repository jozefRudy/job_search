import { createSignal } from "solid-js";
import { Card } from "~/components/ui/Card";
import { Icon } from "~/components/ui/Icon";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { Swap } from "~/components/ui/Swap";
import { DevLayout } from "../DevLayout";

function ControlledExample(props: {
  label: string;
  size?: "sm" | "md" | "lg";
  class?: string;
}) {
  const [checked, setChecked] = createSignal(false);
  return (
    <Stack gap="sm" align="center">
      <span class="w-24 text-sm">{props.label}</span>
      <Swap
        checked={checked()}
        onChange={setChecked}
        size={props.size}
        class={props.class}
        on={
          <Row gap="sm" align="center">
            <Icon id="cancel_schedule_send" class="size-[1em]" />
            <span>Applied</span>
          </Row>
        }
        off={
          <Row gap="sm" align="center">
            <Icon id="send" class="size-[1em]" />
            <span>Apply</span>
          </Row>
        }
      />
    </Stack>
  );
}

export default function SwapPage() {
  return (
    <DevLayout title="Swap Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Variants</h2>
          <Card>
            <Card.Body>
              <Row gap="md" align="center" class="flex-wrap">
                <ControlledExample label="Plain" />
                <ControlledExample label="Button" class="btn btn-primary" />
              </Row>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Sizes</h2>
          <Card>
            <Card.Body>
              <Row gap="md" align="center" class="flex-wrap">
                <ControlledExample label="Small" size="sm" />
                <ControlledExample label="Medium" size="md" />
                <ControlledExample label="Large" size="lg" />
              </Row>
            </Card.Body>
          </Card>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
