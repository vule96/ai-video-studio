import { useMutation, useQueryClient } from '@tanstack/react-query';
import { FileText } from 'lucide-react';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { type SubmitEvent, useState } from 'react';

import type { Format } from '@/lib/bindings/Format';
import type { SourceKind } from '@/lib/bindings/SourceKind';
import { api, errorMessage } from '@/lib/ipc';
import { openPdf, pdfText } from '@/lib/pdf';
import { keys } from '@/lib/queries';
import type { Navigate } from '@/route';
import { Button, ErrorText, Field, NumberInput, Segmented, TextArea } from '@/ui';

interface LoadedPdf {
  name: string;
  doc: PDFDocumentProxy;
}

export function NewProjectScreen({ navigate }: { navigate: Navigate }) {
  const qc = useQueryClient();
  const [kind, setKind] = useState<SourceKind>('prompt');
  const [prompt, setPrompt] = useState('');
  const [pdf, setPdf] = useState<LoadedPdf | null>(null);
  const [pdfError, setPdfError] = useState<string | null>(null);
  const [pages, setPages] = useState({ from: 1, to: 1 });
  const [shotCount, setShotCount] = useState(6);
  const [format, setFormat] = useState<Format>('landscape');
  const [language, setLanguage] = useState<'vi' | 'en'>('vi');

  const create = useMutation({
    mutationFn: async () => {
      const text = kind === 'pdf' && pdf ? await pdfText(pdf.doc, pages.from, pages.to) : prompt;
      const label = kind === 'pdf' && pdf ? pdf.name.replace(/\.pdf$/i, '') : prompt.slice(0, 80);
      return api.projectCreate({ sourceKind: kind, sourceLabel: label, text, language, format, shotCount });
    },
    onSuccess: (project) => {
      void qc.invalidateQueries({ queryKey: keys.projects });
      navigate({ name: 'project', id: project.id });
    },
  });

  async function pickPdf(file: File | undefined) {
    if (!file) return;
    setPdfError(null);
    try {
      const doc = await openPdf(file);
      setPdf({ name: file.name, doc });
      setPages({ from: 1, to: doc.numPages });
    } catch (e) {
      setPdf(null);
      setPdfError(`Không mở được file PDF: ${errorMessage(e)}`);
    }
  }

  const ready = kind === 'prompt' ? prompt.trim().length > 0 : pdf !== null && pages.from <= pages.to;

  function submit(e: SubmitEvent<HTMLFormElement>) {
    e.preventDefault();
    if (ready) create.mutate();
  }

  return (
    <form onSubmit={submit} className="mx-auto flex max-w-2xl flex-col gap-6 px-8 py-8">
      <header className="flex flex-col gap-1">
        <h1 className="text-[22px] font-medium">Dự án mới</h1>
        <p className="text-dim">
          LLM trên máy bạn đọc nội dung, chọn nhân vật và chia thành các shot. Bạn sửa lại được trước khi vẽ.
        </p>
      </header>

      <Segmented
        label="Nguồn nội dung"
        value={kind}
        onChange={setKind}
        options={[
          { value: 'prompt', label: 'Mô tả' },
          { value: 'pdf', label: 'File PDF' },
        ]}
      />

      {kind === 'prompt' ? (
        <Field
          label="Câu chuyện"
          hint="Càng cụ thể về nhân vật, bối cảnh, thời đại thì storyboard càng sát ý."
        >
          {(id) => (
            <TextArea
              id={id}
              rows={6}
              value={prompt}
              placeholder="Một cô bé bán diêm ở Sài Gòn những năm 1960, đêm giao thừa…"
              onChange={(e) => {
                setPrompt(e.target.value);
              }}
            />
          )}
        </Field>
      ) : (
        <div className="flex flex-col gap-3">
          <label
            className="flex cursor-pointer items-center gap-3 rounded-md border border-dashed border-line px-4 py-5 hover:border-dim"
            onDragOver={(e) => {
              e.preventDefault();
            }}
            onDrop={(e) => {
              e.preventDefault();
              void pickPdf(e.dataTransfer.files[0]);
            }}
          >
            <FileText size={20} className="text-dim" />
            <span className="flex-1">
              {pdf ? (
                <>
                  <span className="font-medium">{pdf.name}</span>
                  <span className="text-dim">, {pdf.doc.numPages} trang</span>
                </>
              ) : (
                <span className="text-dim">Kéo file PDF vào đây, hoặc bấm để chọn</span>
              )}
            </span>
            <input
              type="file"
              accept="application/pdf"
              className="sr-only"
              onChange={(e) => void pickPdf(e.target.files?.[0])}
            />
          </label>
          {pdfError && <ErrorText>{pdfError}</ErrorText>}
          {pdf && (
            <div className="flex items-end gap-3">
              <Field label="Từ trang">
                {(id) => (
                  <NumberInput
                    id={id}
                    min={1}
                    max={pages.to}
                    value={pages.from}
                    className="w-24"
                    onValueChange={(from) => {
                      setPages((p) => ({ ...p, from }));
                    }}
                  />
                )}
              </Field>
              <Field label="Đến trang">
                {(id) => (
                  <NumberInput
                    id={id}
                    min={pages.from}
                    max={pdf.doc.numPages}
                    value={pages.to}
                    className="w-24"
                    onValueChange={(to) => {
                      setPages((p) => ({ ...p, to }));
                    }}
                  />
                )}
              </Field>
              <p className="pb-1.5 text-[12px] text-faint">
                Sách dài sẽ được tóm tắt từng phần trước, mỗi ~6.000 ký tự là một lượt gọi LLM.
              </p>
            </div>
          )}
        </div>
      )}

      <div className="grid grid-cols-3 gap-4">
        <Field label="Số shot">
          {(id) => <NumberInput id={id} min={1} max={40} value={shotCount} onValueChange={setShotCount} />}
        </Field>
        <div className="flex flex-col gap-1">
          <span className="text-[13px] text-dim">Khung hình</span>
          <Segmented
            label="Khung hình"
            value={format}
            onChange={setFormat}
            options={[
              { value: 'landscape', label: 'Ngang 16:9' },
              { value: 'portrait', label: 'Dọc 9:16' },
            ]}
          />
        </div>
        <div className="flex flex-col gap-1">
          <span className="text-[13px] text-dim">Lời dẫn</span>
          <Segmented
            label="Ngôn ngữ lời dẫn"
            value={language}
            onChange={setLanguage}
            options={[
              { value: 'vi', label: 'Tiếng Việt' },
              { value: 'en', label: 'English' },
            ]}
          />
        </div>
      </div>

      {create.error && <ErrorText>{errorMessage(create.error)}</ErrorText>}

      <div className="flex gap-2">
        <Button type="submit" variant="primary" disabled={!ready || create.isPending}>
          {create.isPending ? 'Đang tạo…' : 'Tạo storyboard'}
        </Button>
        <Button
          onClick={() => {
            navigate({ name: 'projects' });
          }}
        >
          Huỷ
        </Button>
      </div>
    </form>
  );
}
