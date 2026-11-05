# Quota Station

[English](README.md) · **Tiếng Việt**

Quota Station là ứng dụng trên thanh menu macOS và khay hệ thống Windows dành cho lập trình viên
dùng **Claude Code**, **Codex** và **Antigravity**. App đọc hạn mức trực tiếp trên máy bạn,
chuyển nhanh giữa các tài khoản bằng một cú nhấp (hoặc tự chuyển khi tài khoản Claude hết
quota), và quản lý chung MCP server, skill, plugin cùng file hướng dẫn của các agent ở một nơi.
Giao diện có tiếng Việt và tiếng Anh, hỗ trợ macOS 13+ và Windows 10/11 (x64) — tải về từ
[Releases](https://github.com/shin315/quota-station/releases/latest) hoặc xem mục
[Cài đặt](#cài-đặt).

<p align="center">
  <img src="docs/screenshots/vi/popover.png" width="360" alt="Popover mở từ thanh menu macOS hoặc khay hệ thống Windows"><br>
  <sub><b>Popover</b> (mở khi nhấp biểu tượng trên thanh menu macOS hoặc khay hệ thống Windows): thanh hạn mức và thời gian đặt lại của Claude, Antigravity và Codex, cùng mục <code>Tự chuyển khi hết quota</code> ở dưới cùng.</sub>
</p>

## Tính năng

- **Tự chuyển khi hết quota**: Khi có từ hai tài khoản Claude trở lên (Pro, Max hoặc Team), bật
  `Tự chuyển khi hết quota` ở tab `Claude` hoặc dưới chân popover: khi tài khoản đang dùng chạm
  90% cửa sổ 5 giờ hoặc tuần (`Cài đặt › Dịch vụ`), Claude Code tự chuyển sang tài khoản còn
  nhiều quota nhất trong khi các phiên đang mở vẫn chạy tiếp. App kiểm tra mỗi 3 phút, gửi thông
  báo sau mỗi lần chuyển và có nút `Tạm dừng` bất cứ lúc nào.
- **Thanh menu, popover và `Tổng quan`**: Thanh menu hiện đồng hồ nhỏ cùng tài khoản đang dùng
  và phần trăm đã dùng (`lan.acme1 74%`), nhấp trái mở popover xem mọi cửa sổ hạn mức. Trong cửa
  sổ chính, tab `Tổng quan` gom tất cả tài khoản Claude, Antigravity và Codex vào một chỗ với
  nút `Chuyển` trên từng tài khoản đang không dùng.
- **Tài khoản Claude, Codex và Antigravity**: Lưu và chuyển giữa nhiều tài khoản Claude (đăng
  nhập trình duyệt, lấy tài khoản Claude Code đang dùng, hoặc dán setup-token), nhiều tài khoản
  ChatGPT và Custom API trong `config.toml` cho Codex (giữ đăng nhập ChatGPT để dùng plugin, kèm
  `Tự đánh thức ChatGPT`), và nhiều tài khoản Google cho Antigravity.app cùng `agy` (đăng nhập
  bằng OAuth client của Antigravity đã cài trên cùng máy, không phải cài đặt gì;
  [chi tiết](docs/how-it-works.md#the-oauth-client-file)).
- **MCP server, skill, plugin và file hướng dẫn**: Bật/tắt MCP server và skill cho từng agent
  (`Claude Code`, `Codex`, `Antigravity`) trên một bảng chung, cài hoặc cập nhật skill từ
  GitHub, xem `Plugin` đã cài, và theo dõi `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` (`File hướng
  dẫn`). Thay đổi được gom vào một bước xem trước, tự sao lưu trước khi ghi và hoàn tác được
  ngay.
- **Bộ cấu hình và Sao lưu**: Lưu nhóm MCP server và skill thành các `Bộ cấu hình` để áp dụng
  hoặc hoàn tác cho toàn máy trong một bước. Sao lưu cấu hình lên iCloud Drive (hoặc thư mục tự
  chọn) rồi khôi phục từng mục trên bất kỳ máy Mac nào — gói sao lưu không bao giờ chứa khoá API.
- **Thống kê, Cập nhật Agent, Lịch sử và Thông báo**: Xem số token Claude Code và Codex đã dùng
  cùng chi phí API tương đương (`Thống kê`), kiểm tra và cập nhật CLI của các agent (`Cập
  nhật`), xem `Lịch sử` chuyển tài khoản, và nhận thông báo khi hạn mức sắp cạn, tài khoản cần
  đăng nhập lại hoặc có biến môi trường (như `ANTHROPIC_API_KEY`) đè đăng nhập.
- **Chủ đề, ẩn email và Công cụ nâng cao**: Chọn giao diện sáng/tối, sáu bộ chủ đề, và tắt `Hiện
  email` khi chụp màn hình. `Cài đặt › Nâng cao › Hiện phần nâng cao` mở thêm trình sửa luật xoay
  vòng riêng, đồng bộ luật và cấu hình agent sang máy Mac khác qua SSH (`Chép sang Mac khác`),
  cùng `Cập nhật từ mã nguồn`.

<p align="center">
  <img src="docs/screenshots/vi/claude.png" width="760" alt="Tab Claude trong cửa sổ chính"><br>
  <sub><b>Cửa sổ chính › Tab Claude</b>: danh sách bốn tài khoản Claude với hạn mức 5 giờ, Tuần và Fable tuần, bên dưới là mục <code>Tự chuyển khi hết quota</code>.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/vi/overview.png" width="760" alt="Tab Tổng quan trong cửa sổ chính"><br>
  <sub><b>Cửa sổ chính › Tab Tổng quan</b>: toàn bộ tài khoản Claude, Antigravity và Codex trên một màn hình, kèm thanh điều hướng bên trái.</sub>
</p>

<p align="center">
  <img src="docs/screenshots/vi/mcp.png" width="760" alt="Tab MCP trong cửa sổ chính"><br>
  <sub><b>Cửa sổ chính › Tab MCP</b>: quản lý chung các MCP server của Claude Code, Codex và Antigravity với ô đánh dấu theo từng agent, xem trước và hoàn tác.</sub>
</p>

<sub>Tất cả ảnh chụp đều dùng dữ liệu thử nghiệm giả lập, không dùng tài khoản thật. Mô tả đầy đủ từng tính năng: <a href="docs/features.md">docs/features.md</a>.</sub>

## Cài đặt

Tải ứng dụng từ [bản phát hành mới nhất](https://github.com/shin315/quota-station/releases/latest)
cho **macOS 13+** (Apple Silicon hoặc Intel) hoặc **Windows 10/11 (x64)**.

- **macOS (Homebrew)**: `brew install --cask shin315/tap/quota-station`
- **macOS (DMG)**: Tải `Quota.Station_<version>_universal.dmg`, mở file rồi kéo `Quota Station`
  vào `Applications` (đã ký Developer ID và được Apple notarize; chọn `Mở` / `Open` ở lần chạy
  đầu).
- **macOS (từ mã nguồn)**: Trên máy Mac có Rust 1.85+ và Node.js 20+, clone repo và chạy
  `scripts/install.sh` ([docs/development.md](docs/development.md#build-and-run)).
- **Windows**: Tải `Quota.Station_<version>_x64-setup.exe` rồi chạy (chỉ cài cho user hiện tại,
  không cần quyền quản trị). Do bộ cài chưa ký số: chọn `Giữ lại` / `Keep` (`Keep anyway` trong
  Edge) nếu trình duyệt cảnh báo, và ở SmartScreen bấm `More info` (`Thông tin thêm`) › `Run
  anyway` (`Vẫn chạy`).

Bật `Cài đặt › Chung › Mở khi đăng nhập` nếu muốn app tự khởi động cùng máy. Đang chuyển từ công
cụ xoay vòng Claude cũ? Xem [hướng dẫn cutover](docs/claude-accounts.md#cutover).

## Quyền riêng tư và an toàn

- Dữ liệu hạn mức được đọc trực tiếp trên máy bạn từ endpoint của từng dịch vụ bằng chính phiên
  đăng nhập sẵn có của agent — không cần tài khoản riêng, không telemetry, không máy chủ trung
  gian.
- App không bao giờ sửa file cấu hình của agent khi chưa qua bước xem trước và xác nhận; mỗi lần
  ghi đều giữ bản sao lưu và tự dừng nếu file bị thay đổi sau khi xem trước.
- Token đăng nhập cùng khoá API được lưu trong Keychain của macOS (hoặc thư mục cục bộ giới hạn
  quyền / mã hoá bởi Windows) và không bao giờ xuất hiện trên cửa sổ, trong log hay gói sao lưu.

Chi tiết về bảo mật và lưu ý điều khoản dịch vụ có trong [SECURITY.md](SECURITY.md) và
[docs/how-it-works.md](docs/how-it-works.md).

## Cập nhật

Bản tải về kiểm tra GitHub Releases mỗi ngày (`Cài đặt › Chung › Cập nhật`) và báo khi có bản
mới: bấm `Tải và cài` để mở file `.dmg` hoặc `.exe` mới trên trình duyệt rồi cài đè lên bản cũ
(trên macOS thì thoát app trước), hoặc chạy `brew upgrade --cask quota-station`. Bản dựng từ mã
nguồn cập nhật bằng `git pull --ff-only && scripts/install.sh` hoặc `Cập nhật từ mã nguồn` trong
`Cài đặt › Nâng cao`. App không bao giờ tự tải hay tự cài bản cập nhật.

## Gỡ cài đặt

Tắt `Mở khi đăng nhập` (`Cài đặt › Chung`) và `Tự chuyển khi hết quota` (tab `Claude`), rồi thoát
Quota Station. Trên **macOS**, kéo `/Applications/Quota Station.app` vào Thùng rác (hoặc `brew
uninstall --cask quota-station`); trên **Windows**, gỡ `Quota Station` trong `Settings › Apps ›
Installed apps`. Để xoá thêm cài đặt, thư mục tài khoản và các mục trong Keychain / Credential
Manager, xem [Uninstalling and removing data](docs/how-it-works.md#uninstalling-and-removing-data).

## Tài liệu

Các tài liệu chi tiết trong `docs/` (viết bằng tiếng Anh, kèm nhãn giao diện tiếng Việt):

- [docs/features.md](docs/features.md) — Mô tả chi tiết mọi tính năng và màn hình của app
- [docs/how-it-works.md](docs/how-it-works.md) — Cơ chế đọc hạn mức, lưu trữ tài khoản, CLI, các file app đọc/ghi, file OAuth client và cách xoá sạch dữ liệu khi gỡ cài đặt
- [docs/development.md](docs/development.md) — Dựng từ mã nguồn, các build feature, xem trước trên trình duyệt và chạy kiểm thử
- [docs/claude-accounts.md](docs/claude-accounts.md) · [docs/themes.md](docs/themes.md) · [docs/ui-guidelines.md](docs/ui-guidelines.md) · [docs/releasing.md](docs/releasing.md) · [SECURITY.md](SECURITY.md)

---

<sub>MIT, trừ phần tài khoản Antigravity tuỳ chọn (CC BY-NC-SA 4.0). Giấy phép và ghi công bên thứ ba có trong <a href="NOTICE">NOTICE</a>, <a href="LICENSE">LICENSE</a>, <a href="LICENSES/">LICENSES/</a> và <a href="THIRD_PARTY.md">THIRD_PARTY.md</a>.</sub>
