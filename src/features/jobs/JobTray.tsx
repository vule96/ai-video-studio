import { useMutation } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useMemo } from 'react';

import type { JobInfo } from '@/lib/bindings/JobInfo';
import { api, errorMessage } from '@/lib/ipc';
import { isActive, useJobs } from '@/lib/jobs';
import type { Navigate } from '@/route';
import { Progress } from '@/ui';

const RECENT_FINISHED = 4;

/** Queue of GPU jobs. They run one at a time, so position in the list is the run order. */
export function JobTray({ navigate }: { navigate: Navigate }) {
  const byId = useJobs((s) => s.byId);
  const { active, recent } = useMemo(() => {
    const all = [...byId.values()];
    return {
      active: all.filter(isActive),
      recent: all
        .filter((j) => !isActive(j))
        .slice(-RECENT_FINISHED)
        .reverse(),
    };
  }, [byId]);

  if (active.length === 0 && recent.length === 0) {
    return <p className="px-4 py-3 text-[12px] text-faint">Không có việc nào đang chạy.</p>;
  }

  return (
    <section aria-label="Hàng đợi" className="flex flex-col gap-3 px-3 py-3">
      {[...active, ...recent].map((job) => (
        <JobRow key={job.id} job={job} navigate={navigate} />
      ))}
    </section>
  );
}

function JobRow({ job, navigate }: { job: JobInfo; navigate: Navigate }) {
  const cancel = useMutation({ mutationFn: () => api.jobCancel(job.id) });
  const active = isActive(job);

  return (
    <div className="flex flex-col gap-1.5 text-[12px]">
      <div className="flex items-start gap-2">
        <button
          type="button"
          className="min-w-0 flex-1 text-left leading-snug text-ink hover:underline"
          onClick={() => {
            navigate({ name: 'project', id: job.projectId });
          }}
        >
          {job.label}
        </button>
        {active && (
          <button
            type="button"
            aria-label="Huỷ"
            title="Huỷ"
            disabled={cancel.isPending}
            onClick={() => {
              cancel.mutate();
            }}
            className="text-faint hover:text-alert"
          >
            <X size={14} />
          </button>
        )}
      </div>
      {job.status === 'running' && <Progress done={job.done} total={job.total} />}
      <p className={job.status === 'failed' ? 'whitespace-pre-wrap text-alert' : 'text-faint'}>
        {job.status === 'failed' ? job.error : statusText(job)}
      </p>
      {cancel.error && <p className="text-alert">{errorMessage(cancel.error)}</p>}
    </div>
  );
}

function statusText(job: JobInfo): string {
  switch (job.status) {
    case 'queued':
      return 'Đang chờ';
    case 'running':
      return job.message;
    case 'done':
      return 'Xong';
    case 'cancelled':
      return 'Đã huỷ';
    case 'failed':
      return 'Lỗi';
  }
}
