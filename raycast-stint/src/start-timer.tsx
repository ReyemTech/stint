import {
  Form,
  ActionPanel,
  Action,
  Toast,
  showToast,
  popToRoot,
} from "@raycast/api";
import { useState, useEffect } from "react";
import { stint } from "./lib/stint";
import type { ProjectDTO, TaskDTO, EntryDTO } from "./lib/types";

interface FormValues {
  description: string;
  project_id: string;
  task_id: string;
  billable: boolean;
}

export default function Command() {
  const [projects, setProjects] = useState<ProjectDTO[]>([]);
  const [tasks, setTasks] = useState<TaskDTO[]>([]);
  const [selectedProject, setSelectedProject] = useState<string>("");
  const [loadingProjects, setLoadingProjects] = useState(true);
  const [loadingTasks, setLoadingTasks] = useState(false);

  useEffect(() => {
    stint<ProjectDTO[]>("projects", "list")
      .then((list) => setProjects(list.filter((p) => !p.archived)))
      .catch((e) =>
        showToast({
          style: Toast.Style.Failure,
          title: "Failed to load projects",
          message: String(e),
        }),
      )
      .finally(() => setLoadingProjects(false));
  }, []);

  useEffect(() => {
    if (!selectedProject) {
      setTasks([]);
      return;
    }
    setLoadingTasks(true);
    stint<TaskDTO[]>("projects", "list-tasks", selectedProject)
      .then((list) => setTasks(list.filter((t) => !t.done)))
      .catch(() => setTasks([]))
      .finally(() => setLoadingTasks(false));
  }, [selectedProject]);

  async function handleSubmit(values: FormValues) {
    try {
      const args = ["start", "--description", values.description];
      if (values.project_id) args.push("--project", values.project_id);
      if (values.task_id) args.push("--task", values.task_id);
      if (values.billable) args.push("--billable");
      const entry = await stint<EntryDTO>(...args);
      await showToast({
        style: Toast.Style.Success,
        title: `Tracking '${entry.description}'`,
      });
      await popToRoot();
    } catch (e) {
      await showToast({
        style: Toast.Style.Failure,
        title: "Failed to start timer",
        message: String(e),
      });
    }
  }

  return (
    <Form
      isLoading={loadingProjects}
      actions={
        <ActionPanel>
          <Action.SubmitForm onSubmit={handleSubmit} title="Start Timer" />
        </ActionPanel>
      }
    >
      <Form.TextField
        id="description"
        title="Description"
        placeholder="What are you working on?"
      />
      <Form.Dropdown
        id="project_id"
        title="Project"
        value={selectedProject}
        onChange={setSelectedProject}
      >
        <Form.Dropdown.Item value="" title="(no project)" />
        {projects.map((p) => (
          <Form.Dropdown.Item
            key={p.solidtime_id}
            value={p.solidtime_id}
            title={p.name}
          />
        ))}
      </Form.Dropdown>
      <Form.Dropdown id="task_id" title="Task" isLoading={loadingTasks}>
        <Form.Dropdown.Item value="" title="(no task)" />
        {tasks.map((t) => (
          <Form.Dropdown.Item
            key={t.solidtime_id}
            value={t.solidtime_id}
            title={t.name}
          />
        ))}
      </Form.Dropdown>
      <Form.Checkbox id="billable" label="Billable" defaultValue={false} />
    </Form>
  );
}
