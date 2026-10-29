import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Check, CircleAlert } from 'lucide-react';
import { useState } from 'react';

import type { Settings } from '@/lib/bindings/Settings';
import { api, errorMessage } from '@/lib/ipc';
import { keys, useHealth, useProjectsRoot, useSettings } from '@/lib/queries';
import { Button, ErrorText, Field, Input, NumberInput, Page, Panel, Select } from '@/ui';

/** The distilled default runs 4 steps without CFG; the hint changes when another model is set. */
const DISTILLED_UNET = 'flux-2-klein-4b-fp8.safetensors';

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
    <Page
      title="Cài đặt"
      subtitle={
        root.data && (
          <>
            Dự án lưu tại <span className="font-mono select-all">{root.data}</span>
          </>
        )
      }
      actions={
        <>
          {save.isSuccess && !dirty && <span className="text-[13px] text-ok">Đã lưu</span>}
          <Button
            variant="primary"
            disabled={!dirty || save.isPending}
            onClick={() => {
              save.mutate();
            }}
          >
            Lưu cài đặt
          </Button>
        </>
      }
    >
      <div className="grid grid-cols-[minmax(0,1fr)_360px] items-start gap-8">
        <form
          className="flex min-w-0 flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          {save.error && <ErrorText>{errorMessage(save.error)}</ErrorText>}

          <Panel title="Viết kịch bản và soát hình" hint="Ollama chạy model ngôn ngữ trên máy bạn.">
            <div className="grid grid-cols-3 gap-4">
              <Field label="Địa chỉ Ollama">
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
                label="Model viết kịch bản"
                hint={installed.length > 0 ? undefined : 'Không đọc được danh sách model, gõ tên model.'}
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
              <Field label="Model soát hình" hint="Cần model nhìn được ảnh, ví dụ gemma3:12b.">
                {(id) => (
                  <Select
                    id={id}
                    value={s.visionModel}
                    onChange={(e) => {
                      setS({ ...s, visionModel: e.target.value });
                    }}
                  >
                    <option value="">Như model viết kịch bản</option>
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
          </Panel>

          <Panel title="Vẽ hình" hint="ComfyUI chạy model ảnh FLUX.2 klein trên GPU.">
            <Field
              label="Địa chỉ ComfyUI"
              hint="Để trống: tự tìm ở cổng 8188 (bản portable) rồi 8000 (bản Desktop)."
            >
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
              <Field
                label="CFG"
                hint={
                  model.unet === DISTILLED_UNET
                    ? 'Bản distilled: 4 bước, CFG 1.'
                    : 'Bản Base: khoảng 20–30 bước, CFG 3–5.'
                }
              >
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
          </Panel>

          <Panel title="Lồng tiếng và dựng phim" hint="VieNeu-TTS đọc lời dẫn, ffmpeg ghép phim.">
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
              <Field
                label="ffmpeg"
                hint="Để chữ ffmpeg: app tự tìm bản winget hoặc Homebrew. Hoặc đường dẫn đầy đủ."
              >
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
          </Panel>
          {/* Enter in any field saves, like the header button. */}
          <button type="submit" hidden />
        </form>

        <aside className="sticky top-6" aria-live="polite">
          <Panel
            title="Sẵn sàng chưa?"
            actions={
              <Button disabled={health.isFetching} onClick={() => void health.refetch()}>
                {health.isFetching ? 'Đang kiểm tra…' : 'Kiểm tra lại'}
              </Button>
            }
          >
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
          </Panel>
        </aside>
      </div>
    </Page>
  );
}
