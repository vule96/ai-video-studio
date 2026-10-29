# AI Video Studio

App desktop cho Windows và macOS: từ **một câu mô tả** hoặc **một file PDF**, app viết storyboard, dựng nhân vật nhất quán rồi vẽ từng khung hình. Mọi AI chạy trên máy bạn, không tốn phí API.

```
Mô tả / PDF ─► 1 Kịch bản ─► 2 Nhân vật ─► 3 Khung hình ─► 4 Dựng phim
               Ollama          ComfyUI        ComfyUI          VieNeu-TTS + ffmpeg
               nhân vật,       3 phương án,   mỗi shot vẽ theo lời dẫn, zoom/pan,
               shot, lời dẫn   khoanh 1 ảnh   ảnh tham chiếu   ghép MP4 + phụ đề .srt
```

## Cần cài

| Thứ                                    | Ghi chú                                                                           |
| -------------------------------------- | --------------------------------------------------------------------------------- |
| Node.js 22.11+                         |                                                                                   |
| Rust (qua [rustup](https://rustup.rs)) | Mặc định là đủ                                                                    |
| **Windows**: Microsoft C++ Build Tools | Chọn "Desktop development with C++". WebView2 có sẵn trên Windows 11              |
| **macOS**: Xcode Command Line Tools    | `xcode-select --install`                                                          |
| [Ollama](https://ollama.com)           | Rồi `ollama pull qwen3:8b`                                                        |
| ComfyUI                                | Windows: bản Portable NVIDIA (GitHub ComfyUI → Releases). macOS: ComfyUI Desktop  |
| Model vẽ hình (~12.5 GB)               | `scripts/download-models.ps1` (Windows) hoặc `scripts/download-models.sh` (macOS) |
| ffmpeg                                 | Windows: `winget install Gyan.FFmpeg`. macOS: `brew install ffmpeg`               |
| Python 3.10+                           | Cho VieNeu-TTS; `scripts/start-tts.ps1` tự tạo môi trường và cài lần đầu          |

Tải model, ví dụ trên Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\download-models.ps1 -ComfyModels "D:\ComfyUI_windows_portable\ComfyUI\models"
```

Model mặc định là **FLUX.2 [klein] 4B** bản FP8 (Apache 2.0): một model vừa sinh ảnh từ chữ, vừa vẽ nhân vật vào cảnh mới theo ảnh tham chiếu, chạy 4 bước, vừa card 16 GB.

## Chạy

```bash
npm install
npm run app          # lần đầu Rust biên dịch mất vài phút
```

Mở ComfyUI (`run_nvidia_gpu.bat`) và Ollama trước. Khi đến bước Dựng phim, mở thêm VieNeu-TTS:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\start-tts.ps1   # macOS: ./scripts/start-tts.sh
```

TTS chạy bằng CPU (khoảng 0.6 lần thời gian thực) để GPU dành cho ComfyUI. Vào **Cài đặt** để xem mọi mục đã có dấu ✓ chưa.

Hoặc mở tất cả bằng một lệnh: script kiểm tra từng dịch vụ, mở cái nào chưa chạy trong cửa sổ riêng, chờ sẵn sàng rồi chạy app.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\start-all.ps1                    # ComfyUI ở %USERPROFILE%\Downloads\ComfyUI_windows_portable
powershell -ExecutionPolicy Bypass -File .\scripts\start-all.ps1 -ComfyDir "D:\ComfyUI_windows_portable" -NoApp
```

```bash
./scripts/start-all.sh                       # macOS: ComfyUI Desktop; hoặc COMFY_DIR=~/ComfyUI ./scripts/start-all.sh
```

Build bộ cài: `npm run app:build`. Tauri chỉ build cho hệ điều hành đang chạy: bản Windows build trên Windows, bản macOS build trên Mac.

## Dùng

1. **Dự án mới**: gõ mô tả hoặc kéo file PDF vào (chọn khoảng trang nếu sách dài), chọn số shot, khung ngang hoặc dọc.
2. **Kịch bản**: LLM viết xong thì sửa phong cách hình, tên, mô tả từng shot, lời dẫn. Thêm, xoá, đổi thứ tự shot bằng các nút trên mỗi shot.
3. **Nhân vật**: tối đa 6 nhân vật, mỗi nhân vật sinh 3 phương án, bấm để khoanh ảnh ưng nhất. Ảnh được khoanh là ảnh tham chiếu. Mỗi shot có tối đa 3 nhân vật.
4. **Khung hình**: vẽ các shot còn thiếu. Bấm vào một shot để sửa mô tả, vẽ lại, hoặc chọn lại một lần vẽ cũ. **Soát khung chưa soát** cho một model nhìn ảnh (ví dụ `gemma3:12b`) kiểm tra từng khung. Chỉ **lỗi nặng** mới đánh dấu ảnh: thiếu nhân vật, một nhân vật bị vẽ hai lần, sai loài, sai trang phục chính, có chữ, lỗi tay chân, sai vật chính của cảnh. Khác biệt nhỏ (dáng, góc máy, sắc màu, phụ kiện nhỏ) là **ghi chú**, không tính là lỗi. **Soát lại tất cả** chấm lại mọi khung, dùng khi app đổi cách soát. Mỗi ảnh giữ kết quả soát của riêng nó, nên soát lại chỉ tốn cho ảnh mới. **Tự sửa N cảnh lỗi** vẽ 3 ảnh mới cho mỗi cảnh lỗi, soát hết, và chỉ đổi sang ảnh mới khi ảnh đó đạt; không ảnh nào đạt thì giữ ảnh cũ.
5. **Dựng phim**: chọn giọng đọc, nhạc nền (tuỳ chọn), có in phụ đề vào hình hay không, rồi bấm Dựng phim. Mỗi shot thành một đoạn dài bằng lời dẫn; máy quay theo gợi ý "chuyển động" (push-in, pull-out, pan, tilt, static, shake, kèm slow/fast). Nhạc tự nhỏ đi khi có lời dẫn, âm lượng chuẩn hoá về −14 LUFS như YouTube/TikTok. Ra file MP4 Full HD và phụ đề `.srt` cùng tên.

Mọi việc nặng (LLM, vẽ ảnh) vào hàng đợi ở góc dưới bên trái và chạy lần lượt từng việc, vì card 16 GB không chứa nổi hai model cùng lúc. Có thể huỷ bất cứ lúc nào.

Dự án lưu tại thư mục dữ liệu của app (đường dẫn hiện ở cuối màn Cài đặt). Mỗi dự án là một thư mục gồm `project.json`, `source.txt`, `characters/`, `shots/`, `render/` (phim, phụ đề, lời dẫn đã đọc).

## Phát triển

Đọc [AGENTS.md](AGENTS.md): kiến trúc, quy ước và cách làm theo ponytail.

```bash
npm run check        # TS: typecheck + lint + test
npm run check:rust   # Rust: fmt + clippy + test
npm run bindings     # sinh lại type TS sau khi đổi struct Rust
```

## Hướng phát triển

| Bản                 | Nội dung                                                                                                                                                                                                |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1                 | Dự án, Kịch bản, Nhân vật, Khung hình, Cài đặt, hàng đợi job                                                                                                                                            |
| 0.2                 | Bước 4 Dựng phim: lời dẫn VieNeu-TTS, zoom/pan theo gợi ý camera, chuyển cảnh mờ, phụ đề .srt, MP4 bằng ffmpeg                                                                                          |
| 0.2.1               | 6 nhân vật; góc máy đa dạng; nhạc nền tự nhỏ khi có lời; −14 LUFS; phụ đề in vào hình; tốc độ/rung máy; thêm/xoá/sắp xếp shot; soát lỗi khung hình bằng model nhìn ảnh                                  |
| 0.2.2               | Tự sửa cảnh lỗi (vẽ nhiều lần, soát, chỉ đổi khi có ảnh đạt); soát chỉ ảnh mới; kết quả soát theo từng ảnh; sửa dải ảnh bị ép mất trong khung chi tiết                                                  |
| **0.2.3 (bản này)** | Soát chia lỗi nặng (chặn ảnh) và ghi chú nhỏ (chỉ để tham khảo); nút Soát lại tất cả; giao diện mới: khung giữa 1360px, bảng bên cố định, nút chính màu mực, vàng chỉ đánh dấu lựa chọn                 |
| 0.3: Chuyển động AI | Biến khung hình thành clip bằng Wan 2.2 image-to-video, chọn shot nào dùng clip AI, shot nào dùng zoom/pan                                                                                              |
| Sau đó              | Whisper nghe lại lời dẫn; xuất cùng lúc 16:9 và 9:16; train LoRA cho nhân vật dùng lâu dài; preset phong cách; tiến độ từng bước vẽ (WebSocket ComfyUI); tự mở ComfyUI/Ollama; tải model ngay trong app |
