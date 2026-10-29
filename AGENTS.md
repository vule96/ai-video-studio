# AGENTS.md

Hướng dẫn cho agent (Claude Code, Codex…) làm việc trong repo này. Code, comment, commit message viết tiếng Anh; chữ hiển thị cho người dùng viết tiếng Việt.

## App làm gì

Desktop app (Tauri 2, Windows + macOS) biến một câu mô tả hoặc file PDF thành storyboard có nhân vật nhất quán, vẽ từng khung hình, rồi dựng thành phim có lời dẫn. Mọi AI chạy local qua HTTP:

- **Ollama**: viết storyboard (nhân vật + shot) dạng JSON có schema; một model nhìn ảnh soát khung hình.
- **ComfyUI**: vẽ ảnh bằng FLUX.2 klein 4B. Nhân vật giữ mặt nhờ ảnh tham chiếu (multi-reference). Clip chuyển động bằng Wan 2.2 image-to-video 14B (lightx2v 4 bước) + Real-ESRGAN + RIFE, workflow ở `pipeline/workflow.rs`; tên file model là hằng số ở đó.
- **VieNeu-TTS**: đọc lời dẫn qua server OpenAI-compatible (`scripts/start-tts.*`), chạy CPU.
- **ffmpeg** (process, không qua HTTP): zoom/pan từng khung hình hoặc phát clip AI, ghép MP4, trộn nhạc, chuẩn hoá âm lượng, in phụ đề.

App không đóng gói model hay Python. Người dùng tự cài Ollama, ComfyUI, ffmpeg; VieNeu cài bằng script.

## Lệnh

| Lệnh                 | Việc                                                                                                 |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| `npm run app`        | Chạy app ở chế độ dev (Vite + Rust, hot reload UI)                                                   |
| `npm run app:build`  | Build bộ cài cho hệ điều hành đang dùng                                                              |
| `npm run check`      | typecheck + lint + test cho TS. Chạy trước khi commit                                                |
| `npm run check:rust` | rustfmt + clippy `-D warnings` + cargo test                                                          |
| `npm run bindings`   | Sinh lại type TS từ Rust vào `src/lib/bindings/` (ts-rs). Chạy sau khi đổi struct có `#[ts(export)]` |

## Kiến trúc

```
src/                      React 19 + TS strict + Tailwind v4
  App.tsx, route.ts       4 màn hình (dự án có 4 bước), điều hướng bằng union type (không dùng router)
  lib/ipc.ts              Wrapper có type cho mọi Tauri command + event. UI chỉ gọi backend qua đây
  lib/queries.ts          react-query hooks; useBackendEvents nối event Rust vào cache/store
  lib/jobs.ts             zustand store cho job đang chạy
  lib/pdf.ts              pdf.js trích chữ (lazy load)
  lib/bindings/           Type sinh từ Rust. KHÔNG sửa tay
  features/<màn hình>/    Component theo màn hình
  ui.tsx                  Bộ component cơ bản (Button, Field, Input…)
src-tauri/src/
  lib.rs                  Khởi tạo: settings, store, job worker, đăng ký command
  commands.rs             Command Tauri: mỏng, chỉ nối UI với store/jobs
  model.rs                Dữ liệu dự án (project.json) + áp dụng chỉnh sửa từ UI
  store.rs                Lưu dự án trên đĩa: ghi atomic, khoá theo từng dự án
  jobs.rs                 Hàng đợi job: 1 worker, chạy tuần tự (GPU 16 GB không chứa nổi 2 model)
  services/               Client HTTP cho Ollama, ComfyUI, VieNeu-TTS; chạy ffmpeg
  pipeline/               Logic thuần: prompt, parse output LLM, dựng workflow ComfyUI, tham số ffmpeg, xử lý text
  settings.rs, health.rs, error.rs
```

Luồng dữ liệu: UI gọi command → command ghi store hoặc đẩy job → worker chạy job, ghi store, phát event `job` / `project-changed` → UI cập nhật cache.

## Quy ước

- **Một kiểu lỗi**: mọi command trả `AppResult<T>`; UI nhận `{ kind, message }`, hiện bằng `errorMessage()`. Thông báo lỗi tiếng Việt, nói rõ cách sửa.
- **Dữ liệu từ UI và từ LLM đều không tin được**: id dự án được kiểm trước khi ghép vào đường dẫn (`ProjectStore::dir`); patch kiểm id nhân vật/shot/take (`Project::apply`); output LLM qua `storyboard::normalize`.
- **Không ghi đè dữ liệu**: UI gửi patch theo từng phần (`ProjectPatch`), không gửi cả project, để không xoá ảnh mà job vừa thêm. Ghi file qua `write_atomic`.
- **Webview không có quyền đọc file**: chỉ `core:default`. Ảnh hiển thị qua asset protocol, scope giới hạn trong thư mục dự án.
- Không `unwrap()`/`expect()` ngoài test và `main`.
- TS: strict tối đa (`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`…), ESLint strictTypeChecked, Prettier, import sort.
- Giao diện: nền graphite trung tính (ảnh không bị ám màu khi duyệt), màu vàng "bút sáp" chỉ đánh dấu lựa chọn của người dùng (shot đang mở, ảnh đã chọn, bước hiện tại); nút chính màu mực. Mỗi màn hình là một khung giữa 1360px (`Page`), nhóm trường dùng `Panel`, hàng hành động dùng `Toolbar` trong `src/ui.tsx`. Token màu ở `src/app.css`.

## Ponytail

Repo này theo [ponytail](https://github.com/DietrichGebert/ponytail) (MIT, © 2026 DietrichGebert):

> You are a lazy senior developer. The best code is the code never written. You solve the whole problem with the least new code. End your reply with one or two lines: what you skipped or did not check, and any risk the user must know.

**Trước khi viết**: đọc task và code nó chạm tới. Liệt kê mọi chỗ thay đổi phải tới: caller, test, fixture, config, export. Kiểm tra thay đổi có thể làm hỏng gì cho người dùng.

**Thay đổi nhỏ nhất mà đủ**, lấy phương án đầu tiên dùng được:

1. Có cần tồn tại không? Bỏ tính năng, tuỳ chọn không ai yêu cầu, và nêu tên chúng trong một dòng.
2. Codebase đã có chưa? Dùng như code xung quanh đang dùng.
3. Thư viện chuẩn hoặc tính năng nền tảng có không?
4. Dependency đã cài có không? Không thêm dependency cho vài dòng code.
5. Một dòng dễ đọc được không?
6. Nếu không: code tối thiểu chạy được.

- Lười với lời giải, không lười với thay đổi: làm đủ mọi phần task cần, kể cả caller và test bị ảnh hưởng.
- Không abstraction, wrapper, option, config "để sau" mà không ai yêu cầu. Giữ cấu trúc sẵn có: các lớp, interface, quy ước.
- Comment chỉ giải thích _vì sao_, một dòng.
- Sửa bug: grep mọi caller trước, sửa gốc một lần ở code dùng chung.
- Logic không tầm thường (rẽ nhánh, vòng lặp, parser, bảo mật) phải kèm một test nhỏ.
- Đường tắt có giới hạn đã biết thì ghi: `// shortcut: <giới hạn>, <khi nào nâng cấp>`.

**Không bao giờ cắt**: kiểm tra dữ liệu ở ranh giới tin cậy, xử lý lỗi tránh mất dữ liệu, bảo mật, accessibility, điều người dùng đã yêu cầu.
