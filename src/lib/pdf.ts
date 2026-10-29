import type { PDFDocumentProxy } from 'pdfjs-dist';

// pdf.js is large; it loads only when the user actually picks a PDF.
async function loadPdfjs() {
  const [pdfjs, worker] = await Promise.all([
    import('pdfjs-dist'),
    import('pdfjs-dist/build/pdf.worker.min.mjs?url'),
  ]);
  pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
  return pdfjs;
}

export async function openPdf(file: File): Promise<PDFDocumentProxy> {
  const pdfjs = await loadPdfjs();
  return pdfjs.getDocument({ data: new Uint8Array(await file.arrayBuffer()) }).promise;
}

/** Text of pages `from`..`to` (1-based, inclusive). Cleaning happens in Rust (`clean_text`). */
export async function pdfText(doc: PDFDocumentProxy, from: number, to: number): Promise<string> {
  const pages: string[] = [];
  for (let n = Math.max(1, from); n <= Math.min(to, doc.numPages); n++) {
    const page = await doc.getPage(n);
    const content = await page.getTextContent();
    let text = '';
    for (const item of content.items) {
      if ('str' in item) text += item.str + (item.hasEOL ? '\n' : '');
    }
    pages.push(text);
    page.cleanup();
  }
  return pages.join('\n\n');
}
