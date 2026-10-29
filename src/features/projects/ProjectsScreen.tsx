import { Plus } from 'lucide-react';

import type { ProjectSummary } from '@/lib/bindings/ProjectSummary';
import { errorMessage, projectFileUrl } from '@/lib/ipc';
import { useProjects, useProjectsRoot } from '@/lib/queries';
import type { Navigate } from '@/route';
import { Button, ErrorText } from '@/ui';

export function ProjectsScreen({ navigate }: { navigate: Navigate }) {
  const projects = useProjects();
  const root = useProjectsRoot();

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6 px-8 py-8">
      <header className="flex items-center justify-between">
        <h1 className="text-[22px] font-medium">Dự án</h1>
        <Button
          variant="primary"
          onClick={() => {
            navigate({ name: 'new' });
          }}
        >
          <Plus size={15} /> Dự án mới
        </Button>
      </header>

      {projects.error && <ErrorText>{errorMessage(projects.error)}</ErrorText>}

      {projects.data?.length === 0 && (
        <div className="flex flex-col items-start gap-3 rounded-lg border border-dashed border-line px-6 py-10">
          <p className="text-[16px]">Chưa có dự án nào.</p>
          <p className="max-w-prose text-dim">
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

      <ul className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-x-5 gap-y-7">
        {projects.data?.map((p) => (
          <li key={p.id}>
            <ProjectCard project={p} root={root.data} navigate={navigate} />
          </li>
        ))}
      </ul>
    </div>
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
    <button
      type="button"
      className="group flex w-full flex-col gap-2 text-left"
      onClick={() => {
        navigate({ name: 'project', id: project.id });
      }}
    >
      <div className="aspect-video w-full overflow-hidden rounded-md border border-line bg-rail group-hover:border-dim">
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
  );
}
