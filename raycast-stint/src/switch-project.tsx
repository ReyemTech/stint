import {
  Form,
  ActionPanel,
  Action,
  showToast,
  Toast,
  popToRoot,
} from "@raycast/api";
import { useEffect, useState } from "react";
import { stint } from "./lib/stint";
import type { ProjectDTO, EntryDTO } from "./lib/types";

export default function Command() {
  const [projects, setProjects] = useState<ProjectDTO[]>([]);
  const [loading, setLoading] = useState(true);
  const [current, setCurrent] = useState<EntryDTO | null>(null);

  useEffect(() => {
    Promise.all([
      stint<ProjectDTO[]>("projects", "list"),
      stint<EntryDTO | null>("current"),
    ])
      .then(([p, c]) => {
        setProjects(p.filter((x) => !x.archived));
        setCurrent(c);
      })
      .catch((e) =>
        showToast({
          style: Toast.Style.Failure,
          title: "Failed to load",
          message: String(e),
        }),
      )
      .finally(() => setLoading(false));
  }, []);

  async function handleSubmit(values: { project_id: string }) {
    if (!current) {
      await showToast({
        style: Toast.Style.Failure,
        title: "No timer to switch from",
      });
      return;
    }
    try {
      await stint("stop");
      await stint(
        "start",
        "--description",
        current.description,
        "--project",
        values.project_id,
      );
      const proj = projects.find((p) => p.solidtime_id === values.project_id);
      await showToast({
        style: Toast.Style.Success,
        title: `Switched to ${proj?.name ?? values.project_id}`,
      });
      await popToRoot();
    } catch (e) {
      await showToast({
        style: Toast.Style.Failure,
        title: "Switch failed",
        message: String(e),
      });
    }
  }

  return (
    <Form
      isLoading={loading}
      actions={
        <ActionPanel>
          <Action.SubmitForm onSubmit={handleSubmit} title="Switch Project" />
        </ActionPanel>
      }
    >
      <Form.Description
        text={
          current
            ? `Currently tracking: ${current.description}`
            : "No active timer."
        }
      />
      <Form.Dropdown id="project_id" title="Project">
        {projects.map((p) => (
          <Form.Dropdown.Item
            key={p.solidtime_id}
            value={p.solidtime_id}
            title={p.name}
          />
        ))}
      </Form.Dropdown>
    </Form>
  );
}
