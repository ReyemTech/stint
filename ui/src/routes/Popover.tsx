import TimerCard from "~/components/TimerCard";

export default function Popover() {
  return (
    <div class="w-[300px] p-3">
      <TimerCard />
      <button
        class="mt-3 w-full rounded border border-zinc-300 py-1 text-xs text-zinc-600 dark:border-zinc-700 dark:text-zinc-300"
        onClick={() => window.location.assign("/#/today")}
      >
        Open main window
      </button>
    </div>
  );
}
