import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2 } from 'lucide-react';
import { useState } from 'react';

import type { ProjectSummary } from '@/lib/bindings/ProjectSummary';
import { api, errorMessage, projectFileUrl } from '@/lib/ipc';
import { keys, useProjects, useProjectsRoot } from '@/lib/queries';
import type { Navigate } from '@/route';
import { Button, ErrorText, Page } from '@/ui';

export function ProjectsScreen({ navigate }: { navigate: Navigate }) {
  const projects = useProjects();
  const root = useProjectsRoot();
  const count = projects.data?.length ?? 0;

  return (
    <Page
      title="Dự án"
      subtitle={count > 0 ? `${String(count)} dự án, mới sửa nhất ở đầu` : undefined}
      actions={
        <Button
          variant="primary"
          onClick={() => {
            navigate({ name: 'new' });
          }}
        >
          <Plus size={15} aria-hidden /> Dự án mới
        </Button>
      }
    >
      {projects.error && <ErrorText>{errorMessage(projects.error)}</ErrorText>}

      {projects.data?.length === 0 && (
        <div className="flex max-w-xl flex-col items-start gap-3 rounded-md bg-sheet px-6 py-8">
          <p className="text-[16px] font-medium">Chưa có dự án nào</p>
          <p className="text-dim">
            Bắt đầu từ một câu mô tả câu chuyện hoặc một file PDF. App sẽ viết storyboard, dựng nhân vật rồi
            vẽ từng khung hình, tất cả chạy trên máy của bạn.
          </p>
          <Button
            variant="primary"
            onClick={() => {
              navigate({ name: 'new' });
            }}
          >
            Tạo dự án đầu tiên
          </Button>
        </div>
      )}

      <ul className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-x-6 gap-y-8">
        {projects.data?.map((p) => (
          <li key={p.id}>
            <ProjectCard project={p} root={root.data} navigate={navigate} />
          </li>
        ))}
      </ul>
    </Page>
  );
}

function ProjectCard({
  project,
  root,
  navigate,
}: {
  project: ProjectSummary;
  root: string | undefined;
  navigate: Navigate;
}) {
  return (
    <div className="flex flex-col gap-2">
      <button
        type="button"
        className="group flex w-full flex-col gap-2 text-left"
        onClick={() => {
          navigate({ name: 'project', id: project.id });
        }}
      >
        <div className="aspect-video w-full overflow-hidden rounded-md bg-well ring-1 ring-line group-hover:ring-edge">
          {project.cover && root ? (
            <img
              src={projectFileUrl(root, project.id, project.cover)}
              alt=""
              className="size-full object-cover"
            />
          ) : (
            <div className="flex size-full items-center justify-center text-[12px] text-faint">
              Chưa có khung hình
            </div>
          )}
        </div>
        <div>
          <p className="truncate font-medium group-hover:underline">{project.title}</p>
          <p className="text-[12px] text-faint">
            {project.shotCount} shot, sửa {new Date(project.updatedAt).toLocaleDateString('vi-VN')}
          </p>
        </div>
      </button>
      <DeleteProject project={project} />
    </div>
  );
}

/** Deletes the whole project folder (images, video, narration) after an inline confirm. */
function DeleteProject({ project }: { project: ProjectSummary }) {
  const qc = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const remove = useMutation({
    mutationFn: () => api.projectDelete(project.id),
    onSuccess: () => {
      qc.removeQueries({ queryKey: keys.project(project.id) });
      void qc.invalidateQueries({ queryKey: keys.projects });
    },
  });
  if (!confirming) {
    return (
      <button
        type="button"
        aria-label={`Xoá dự án ${project.title}`}
        className="flex items-center gap-1 self-start text-[12px] text-faint hover:text-alert"
        onClick={() => {
          setConfirming(true);
        }}
      >
        <Trash2 size={13} aria-hidden /> Xoá
      </button>
    );
  }
  return (
    <div className="flex flex-col gap-1.5 text-[12px]">
      <p className="text-dim">Xoá hẳn dự án này cùng mọi ảnh và video? Không khôi phục được.</p>
      <div className="flex gap-2">
        <Button
          variant="danger"
          disabled={remove.isPending}
          onClick={() => {
            remove.mutate();
          }}
        >
          Xoá dự án
        </Button>
        <Button
          onClick={() => {
            setConfirming(false);
          }}
        >
          Thôi
        </Button>
      </div>
      {remove.error && <ErrorText>{errorMessage(remove.error)}</ErrorText>}
    </div>
  );
}
