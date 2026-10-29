import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Check, CircleAlert } from 'lucide-react';
import { useState } from 'react';

import type { Settings } from '@/lib/bindings/Settings';
import { api, errorMessage } from '@/lib/ipc';
import { keys, useHealth, useProjectsRoot, useSettings } from '@/lib/queries';
import { Button, ErrorText, Field, Input, NumberInput, Select } from '@/ui';

export function SettingsScreen() {
  const settings = useSettings();
  if (!settings.data) return null;
  return <SettingsForm initial={settings.data} />;
}

function SettingsForm({ initial }: { initial: Settings }) {
  const qc = useQueryClient();
  const health = useHealth();
  const root = useProjectsRoot();
  const [s, setS] = useState(initial);
  const dirty = JSON.stringify(s) !== JSON.stringify(initial);

  const save = useMutation({
    mutationFn: () => api.settingsSave(s),
    onSuccess: (saved) => {
      qc.setQueryData(keys.settings, saved);
      void health.refetch();
    },
  });

  const installed = health.data?.ollamaModels ?? [];
  const model = s.imageModel;
  const setModel = (patch: Partial<Settings['imageModel']>) => {
    setS({ ...s, imageModel: { ...model, ...patch } });
  };

  return (
    <div className="mx-auto grid max-w-5xl grid-cols-[1fr_320px] gap-10 px-8 py-8">
      <form
        className="flex flex-col gap-8"
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <h1 className="text-[22px] font-medium">Cài đặt</h1>

        <section className="flex flex-col gap-4">
          <h2 className="font-medium">Viết kịch bản (Ollama)</h2>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Địa chỉ">
              {(id) => (
                <Input
                  id={id}
                  value={s.ollamaUrl}
                  onChange={(e) => {
                    setS({ ...s, ollamaUrl: e.target.value });
                  }}
                />
              )}
            </Field>
            <Field
              label="Model"
              hint={
                installed.length > 0
                  ? 'Các model đã cài trong Ollama (ollama list).'
                  : 'Không đọc được danh sách model, gõ tên model.'
              }
            >
              {(id) =>
                // A plain list of what is installed; free text only when Ollama can't be reached.
                installed.length > 0 ? (
                  <Select
                    id={id}
                    value={s.ollamaModel}
                    onChange={(e) => {
                      setS({ ...s, ollamaModel: e.target.value });
                    }}
                  >
                    {!installed.includes(s.ollamaModel) && (
                      <option value={s.ollamaModel}>{s.ollamaModel} (chưa cài)</option>
                    )}
                    {installed.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </Select>
                ) : (
                  <Input
                    id={id}
                    value={s.ollamaModel}
                    onChange={(e) => {
                      setS({ ...s, ollamaModel: e.target.value });
                    }}
                  />
                )
              }
            </Field>
            <Field
              label="Model soát hình"
              hint="Model nhìn được ảnh, dùng cho nút Soát lỗi khung hình. Ví dụ gemma3:12b."
            >
              {(id) => (
                <Select
                  id={id}
                  value={s.visionModel}
                  onChange={(e) => {
                    setS({ ...s, visionModel: e.target.value });
                  }}
                >
                  <option value="">Dùng model viết kịch bản ({s.ollamaModel})</option>
                  {s.visionModel !== '' && !installed.includes(s.visionModel) && (
                    <option value={s.visionModel}>{s.visionModel} (chưa cài)</option>
                  )}
                  {installed.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          </div>
        </section>

        <section className="flex flex-col gap-4">
          <h2 className="font-medium">Vẽ hình (ComfyUI)</h2>
          <Field label="Địa chỉ" hint="Để trống: tự tìm ở cổng 8188 (bản portable) rồi 8000 (bản Desktop).">
            {(id) => (
              <Input
                id={id}
                value={s.comfyUrl}
                placeholder={health.data?.comfyUrl ?? 'Tự tìm'}
                onChange={(e) => {
                  setS({ ...s, comfyUrl: e.target.value });
                }}
              />
            )}
          </Field>
          <div className="grid grid-cols-3 gap-4">
            <Field label="Diffusion model">
              {(id) => (
                <Input
                  id={id}
                  className="font-mono text-[12.5px]"
                  value={model.unet}
                  onChange={(e) => {
                    setModel({ unet: e.target.value });
                  }}
                />
              )}
            </Field>
            <Field label="Text encoder">
              {(id) => (
                <Input
                  id={id}
                  className="font-mono text-[12.5px]"
                  value={model.textEncoder}
                  onChange={(e) => {
                    setModel({ textEncoder: e.target.value });
                  }}
                />
              )}
            </Field>
            <Field label="VAE">
              {(id) => (
                <Input
                  id={id}
                  className="font-mono text-[12.5px]"
                  value={model.vae}
                  onChange={(e) => {
                    setModel({ vae: e.target.value });
                  }}
                />
              )}
            </Field>
            <Field label="Số bước">
              {(id) => (
                <NumberInput
                  id={id}
                  min={1}
                  max={100}
                  value={model.steps}
                  onValueChange={(steps) => {
                    setModel({ steps });
                  }}
                />
              )}
            </Field>
            <Field label="CFG" hint="Model distilled: giữ 1.">
              {(id) => (
                <NumberInput
                  id={id}
                  min={0}
                  step={0.1}
                  value={model.cfg}
                  onValueChange={(cfg) => {
                    setModel({ cfg });
                  }}
                />
              )}
            </Field>
          </div>
        </section>

        <section className="flex flex-col gap-4">
          <h2 className="font-medium">Lồng tiếng và dựng phim</h2>
          <div className="grid grid-cols-2 gap-4">
            <Field
              label="Địa chỉ VieNeu-TTS"
              hint="Server chạy bằng scripts/start-tts.ps1 (Mac: start-tts.sh)."
            >
              {(id) => (
                <Input
                  id={id}
                  value={s.ttsUrl}
                  onChange={(e) => {
                    setS({ ...s, ttsUrl: e.target.value });
                  }}
                />
              )}
            </Field>
            <Field label="ffmpeg" hint="Tên lệnh nếu đã có trong PATH, hoặc đường dẫn đầy đủ tới ffmpeg.">
              {(id) => (
                <Input
                  id={id}
                  className="font-mono text-[12.5px]"
                  value={s.ffmpeg}
                  onChange={(e) => {
                    setS({ ...s, ffmpeg: e.target.value });
                  }}
                />
              )}
            </Field>
          </div>
        </section>

        {save.error && <ErrorText>{errorMessage(save.error)}</ErrorText>}
        <div className="flex items-center gap-3">
          <Button type="submit" variant="primary" disabled={!dirty || save.isPending}>
            Lưu cài đặt
          </Button>
          {save.isSuccess && !dirty && <span className="text-[13px] text-ok">Đã lưu</span>}
        </div>

        {root.data && (
          <p className="text-[12px] text-faint">
            Dự án lưu tại <span className="font-mono select-all">{root.data}</span>
          </p>
        )}
      </form>

      <aside className="flex flex-col gap-3 self-start rounded-md border border-line p-4" aria-live="polite">
        <div className="flex items-center justify-between">
          <h2 className="font-medium">Sẵn sàng chưa?</h2>
          <Button disabled={health.isFetching} onClick={() => void health.refetch()}>
            {health.isFetching ? 'Đang kiểm tra…' : 'Kiểm tra lại'}
          </Button>
        </div>
        {health.error && <ErrorText>{errorMessage(health.error)}</ErrorText>}
        <ul className="flex flex-col gap-2.5">
          {health.data?.checks.map((c) => (
            <li key={c.name} className="flex gap-2 text-[13px]">
              {c.ok ? (
                <Check size={16} className="mt-0.5 shrink-0 text-ok" aria-label="Ổn" />
              ) : (
                <CircleAlert size={16} className="mt-0.5 shrink-0 text-alert" aria-label="Chưa ổn" />
              )}
              <div className="min-w-0">
                <p className="break-words">{c.name}</p>
                {c.hint && <p className="whitespace-pre-wrap text-faint">{c.hint}</p>}
              </div>
            </li>
          ))}
        </ul>
      </aside>
    </div>
  );
}
