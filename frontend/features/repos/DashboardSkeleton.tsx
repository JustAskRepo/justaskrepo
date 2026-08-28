import Skeleton from "@/components/skeleton";

/** Shape-matched placeholder for the dashboard's first load. */
export default function DashboardSkeleton() {
  return (
    <div className="space-y-10" aria-busy="true" aria-live="polite">
      <span className="sr-only">Loading your repositories…</span>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="glass rounded-2xl p-5">
            <Skeleton className="h-3 w-24" />
            <Skeleton className="mt-4 h-9 w-16" />
            <Skeleton className="mt-4 h-2 w-full" />
          </div>
        ))}
      </div>

      <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
        <Skeleton className="h-11 w-full rounded-full lg:max-w-xs" />
        <div className="flex gap-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-8 w-24 rounded-full" />
          ))}
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="glass rounded-2xl p-5">
            <div className="flex items-center gap-3">
              <Skeleton className="h-10 w-10 rounded-xl" />
              <div className="flex-1">
                <Skeleton className="h-3.5 w-2/3" />
                <Skeleton className="mt-2 h-3 w-16" />
              </div>
              <Skeleton className="h-6 w-20 rounded-full" />
            </div>
            <div className="mt-6 grid grid-cols-2 gap-3">
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-16" />
            </div>
            <Skeleton className="mt-6 h-3 w-32" />
            <div className="mt-5 flex justify-between border-t border-white/8 pt-4">
              <Skeleton className="h-7 w-16 rounded-full" />
              <Skeleton className="h-7 w-20 rounded-full" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
