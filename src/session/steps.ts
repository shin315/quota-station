// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The steps of the guided sessions M1 (the look) and M2 (the core) on a
// Windows PC (plan 4.3), in one sitting with a clean stop after M1. Each
// step says what to do and what the right result looks like; the owner
// never counts, writes down or reports anything. The app keeps the
// evidence itself (src-tauri/src/session.rs): the step, the facts read
// from the model, its self-checks, pictures of its own windows. `covers`
// names the items of the M1 and M2 lists in the bodies of PRs #60 (W5),
// #62 (W4), #63 (W2) and #64 (W3); a test checks that every item is
// covered. The words are Vietnamese only: the session is the owner's.

import type { Model, SettingsGroup, Tab } from '../lib/model';

/** What the Rust side answers about the session (`session::SessionView`). */
export interface SessionView {
  on: boolean;
  step: string | null;
  done: string[];
  /** The app started again after the step on screen began. */
  restarted: boolean;
  zip: { name: string; folder: string } | null;
  /** `claude.exe`, `claude.cmd`, or none. */
  claudeProgram: string | null;
  /** This copy of the app, for a command pasted in a terminal. */
  exe: string | null;
}

/** Pictures the app takes by itself while the step is on screen. */
export type Capture = 'popover' | 'main' | 'toast';
/** A picture the owner asks for with "Chụp sau 3 giây". */
export type Shot = 'tray' | 'main' | 'popover';
/** The self-checks of `session::check`. */
export type Check =
  | 'system'
  | 'machine'
  | 'claudeStore'
  | 'claudeSessions'
  | 'rotation'
  | 'port1455'
  | 'codexConfig'
  | 'codexAccounts'
  | 'secrets'
  | 'mainWindow'
  | 'tray'
  | 'material'
  | 'autostart';
/** A Windows Settings page or a file `session_open` opens. */
export type Page = 'colors' | 'display' | 'taskbar' | 'contrast' | 'startup' | 'notifications' | 'codexConfig';

export interface Ctx {
  model: Model;
  session: SessionView;
}

/** A button under the step that does part of it for the owner. */
export type Help =
  | { kind: 'page'; page: Page; label: string }
  | { kind: 'window'; tab: Tab; group?: SettingsGroup; label: string }
  | { kind: 'shot'; what: Shot; label: string }
  | { kind: 'toast'; label: string }
  | { kind: 'open'; what: 'releases' | 'backup' | 'mcpBackups' | 'notifySettings'; label: string }
  | { kind: 'copy'; text: (c: Ctx) => string; label: string };

type Line = string | ((c: Ctx) => string);

export interface Step {
  id: string;
  part: 1 | 2;
  title: string;
  /** What to do, one action a line. */
  do: Line[];
  /** What the right result looks like. */
  right: Line[];
  captures?: Capture[];
  checks?: Check[];
  help?: Help[];
  /** The step may be left out; the words of its button say when. */
  skip?: string;
  /** Why this PC cannot do the step (it then passes by itself), or null. */
  cannot?: (c: Ctx) => string | null;
  /** The owner signs out here: the step is done once the app started again by itself. */
  restart?: boolean;
  /** Writes the zip: the end of part 1 or 2. */
  end?: 1 | 2;
  /** The items of the PRs' M1 and M2 lists the step checks. */
  covers: string[];
}

/** Every item of the M1 and M2 lists in PRs #60, #62, #63 and #64. */
export const ITEMS = [
  // #63 (W2), M1: tray, popover, hidden icons, DPI, two screens, taskbar edges,
  // main window, material, contrast and theme, machine name, second launch,
  // WebView, opening things.
  'W2.1',
  'W2.2',
  'W2.3',
  'W2.4',
  'W2.5',
  'W2.6',
  'W2.7',
  'W2.8',
  'W2.9',
  'W2.10',
  'W2.11',
  'W2.12',
  // #60 (W5), M1: fonts, title bar, corners, scroll bars, contrast themes,
  // Khay hệ thống, no Keychain, words.
  'W5.1',
  'W5.2',
  'W5.3',
  'W5.4',
  'W5.5',
  'W5.6',
  'W5.7',
  'W5.8',
  // #62 (W4), M2: autostart, off in Settings, toasts, Action Center,
  // notifications off, rotation, token refresh, backup folder, files on
  // demand, diagnostic info.
  'W4.1',
  'W4.2',
  'W4.3',
  'W4.4',
  'W4.5',
  'W4.6',
  'W4.7',
  'W4.8',
  'W4.9',
  'W4.10',
  // #64 (W3), M2: Claude default store, Credential Manager, while it works,
  // session folders, Claude Code found, own secrets, Codex accounts, Codex
  // keyring, Codex processes, agy, variables.
  'W3.1',
  'W3.2',
  'W3.3',
  'W3.4',
  'W3.5',
  'W3.6',
  'W3.7',
  'W3.8',
  'W3.9',
  'W3.10',
  'W3.11',
] as const;

const SHOT_TRAY: Help = { kind: 'shot', what: 'tray', label: 'Chụp góc khay sau 3 giây' };
const SHOT_MAIN: Help = { kind: 'shot', what: 'main', label: 'Chụp cửa sổ chính sau 3 giây' };

/** A Claude account kept and on, but not the one in use: for `claude run`. */
export function spareSlot(model: Model): number | null {
  const row = model.claudeAccounts.accounts.find((a) => a.enabled && !a.active);
  return row ? row.slot : null;
}

const noCodex = ({ model }: Ctx) => (model.codex.present ? null : 'Máy này chưa có Codex, nên bước này tự bỏ qua. Không cần cài gì thêm.');

/** PowerShell's way to run this copy of the app with arguments and wait for it. */
function run(c: Ctx, args: string): string {
  const exe = c.session.exe ?? `${'$'}env:LOCALAPPDATA\\Quota Station\\quota-station.exe`;
  return `& "${exe}" ${args} | Out-Host`;
}

export const STEPS: Step[] = [
  // ===== Phần 1: Nhìn (M1) =====
  {
    id: 'm1-start',
    part: 1,
    title: 'Bắt đầu',
    do: [
      'Khung này là hướng dẫn. Nó luôn nằm trên các cửa sổ khác; nếu nó che chỗ cần xem, kéo nó sang chỗ trống. Lỡ đóng nó thì bấm phải icon khay › Hướng dẫn phiên thử.',
      'Cửa sổ chính của app đang mở ở các bước lần đầu. Làm theo nó: chọn Tiếng Việt, đi qua các bước và bấm nút cuối để xong.',
      'App tự chụp ảnh các cửa sổ của nó và tự ghi kết quả từng bước. Không cần ghi chép gì; chỉ làm theo rồi bấm Xong.',
    ],
    right: [
      'Bước lần đầu có mục “Quyền và khay hệ thống”, và dòng cuối của nó chỉ tới khay ở bên phải thanh taskbar.',
      'Bản thử này chỉ cho phiên hôm nay. Muốn gỡ sau này: Cài đặt Windows › Ứng dụng › Quota Station › Gỡ cài đặt.',
    ],
    captures: ['main'],
    checks: ['system', 'mainWindow'],
    covers: ['W5.6'],
  },
  {
    id: 'm1-tray',
    part: 1,
    title: 'Icon khay',
    do: [
      'Nhìn góc phải dưới thanh taskbar, cạnh đồng hồ. Nếu không thấy icon Quota Station, bấm mũi tên ^ rồi kéo icon ra thanh taskbar.',
      'Bấm nút “Chụp góc khay sau 3 giây” bên dưới, rồi để chuột yên trên icon cho tới khi hiện dòng chữ nhỏ (tooltip).',
    ],
    right: [
      'Icon là một con số (ví dụ 38), nét rõ, chữ trắng trên taskbar tối.',
      'Tooltip bắt đầu bằng tên tài khoản và phần trăm, ví dụ “tài khoản 38%”, rồi tới dòng mô tả.',
      'Khi app chưa đọc xong lần đầu, hoặc khi xoay vòng đã cạn, icon là hình chữ C thay cho con số.',
    ],
    help: [SHOT_TRAY],
    checks: ['tray'],
    covers: ['W2.1'],
  },
  {
    id: 'm1-tray-light',
    part: 1,
    title: 'Taskbar sáng',
    do: [
      'Bấm “Mở Cài đặt › Màu”. Ở “Chọn chế độ” (Choose your mode) chọn Sáng (Light).',
      'Chờ vài giây. Nếu con số chưa đổi màu, bấm phải icon khay › Làm mới.',
      'Bấm “Chụp góc khay sau 3 giây” và để chuột trên icon.',
      'Chọn lại chế độ như lúc đầu (thường là Tối).',
    ],
    right: ['Với taskbar sáng, con số trên icon chuyển sang màu đen, vẫn nét.'],
    help: [{ kind: 'page', page: 'colors', label: 'Mở Cài đặt › Màu' }, SHOT_TRAY],
    checks: ['tray'],
    covers: ['W2.1'],
  },
  {
    id: 'm1-popover',
    part: 1,
    title: 'Popover',
    do: [
      'Bấm chuột trái vào icon khay: popover mở ra.',
      'Bấm lại vào icon: popover đóng, không tự mở lại.',
      'Mở lại, rồi bấm ra chỗ trống trên màn hình: popover đóng.',
      'Mở lại, bấm vào một dòng hoặc nút trong popover để nội dung dài ra hay ngắn lại, rồi đóng.',
    ],
    right: [
      'Popover mở ngay trên icon, cách taskbar một khoảng nhỏ, căn giữa theo icon, nằm trọn trên màn hình, góc bo tròn nhỏ.',
      'Khi nội dung đổi chiều cao, mép dưới vẫn giữ nguyên khoảng cách với taskbar.',
      'Nếu popover có thanh cuộn, thanh cuộn mảnh và mờ.',
    ],
    captures: ['popover'],
    covers: ['W2.2', 'W5.3', 'W5.4'],
  },
  {
    id: 'm1-hidden',
    part: 1,
    title: 'Icon trong mục ẩn',
    do: [
      'Kéo icon Quota Station vào mũi tên ^ (mục icon ẩn).',
      'Bấm ^, rồi bấm icon Quota Station trong ô hiện ra.',
      'Đóng popover. Mở menu Start, gõ Quota Station, bấm Enter để mở app lần nữa.',
      'Bấm ra chỗ trống một lần. Cuối cùng kéo icon từ ^ ra lại thanh taskbar.',
    ],
    right: [
      'Từ mục ẩn: popover mở phía trên taskbar, gần mũi tên ^, nằm trọn trên màn hình.',
      'Mở app lần nữa từ Start: không có icon thứ hai; popover mở cạnh icon khay, cùng chỗ như trên.',
    ],
    captures: ['popover'],
    covers: ['W2.3', 'W2.10'],
  },
  {
    id: 'm1-dpi',
    part: 1,
    title: 'Tỉ lệ màn hình',
    do: [
      'Bấm “Mở Cài đặt › Màn hình”. Ở Tỉ lệ (Scale) chọn 125 %, rồi bấm icon khay mở popover.',
      'Chọn 150 %, rồi bấm icon khay mở popover.',
      'Chọn lại tỉ lệ như lúc đầu. (Nếu lúc đầu đã là 125 % hay 150 %, thử 100 % thay cho nó.)',
    ],
    right: [
      'Ở mỗi tỉ lệ, con số trên icon vẫn nét, không mờ, không vỡ.',
      'Popover giữ đúng bề ngang so với chữ trong nó, và nằm trọn trên màn hình.',
    ],
    help: [{ kind: 'page', page: 'display', label: 'Mở Cài đặt › Màn hình' }],
    captures: ['popover'],
    checks: ['tray'],
    covers: ['W2.4'],
  },
  {
    id: 'm1-screens',
    part: 1,
    title: 'Hai màn hình',
    do: [
      'Ở Cài đặt › Màn hình, cho hai màn hình hai tỉ lệ khác nhau (ví dụ 100 % và 150 %).',
      'Chọn màn hình thứ hai, tick “Đặt làm màn hình chính” (Make this my main display): khay chuyển sang màn hình đó. Bấm icon khay.',
      'Đặt lại màn hình chính và tỉ lệ như lúc đầu, rồi bấm icon khay lần nữa.',
    ],
    right: ['Popover mở trên đúng màn hình có icon, theo tỉ lệ của màn hình đó, không bị cắt.'],
    help: [{ kind: 'page', page: 'display', label: 'Mở Cài đặt › Màn hình' }],
    captures: ['popover'],
    skip: 'Máy chỉ có một màn hình: bỏ qua',
    covers: ['W2.4'],
  },
  {
    id: 'm1-edge',
    part: 1,
    title: 'Taskbar tự ẩn',
    do: [
      'Bấm “Mở Cài đặt › Taskbar”, mở “Hành vi taskbar” (Taskbar behaviors), bật “Tự động ẩn taskbar”.',
      'Đưa chuột xuống đáy màn hình cho taskbar hiện, bấm icon khay.',
      'Trên Windows 10: bấm phải taskbar › Cài đặt taskbar › Vị trí, thử cạnh trên, trái, phải, mỗi lần bấm icon khay.',
      'Tắt tự động ẩn và đặt taskbar lại ở dưới.',
    ],
    right: ['Popover mở sát taskbar, cạnh icon, ở mọi vị trí, nằm trọn trên màn hình.'],
    help: [{ kind: 'page', page: 'taskbar', label: 'Mở Cài đặt › Taskbar' }],
    captures: ['popover'],
    skip: 'Bỏ qua bước này',
    covers: ['W2.5'],
  },
  {
    id: 'm1-main',
    part: 1,
    title: 'Cửa sổ chính',
    do: [
      'Bấm “Mở cửa sổ chính”.',
      'Bấm “Chụp cửa sổ chính sau 3 giây”, rồi để chuột yên trên nút phóng to (ô vuông) ở góc phải trên cho tới khi bảng Snap hiện ra.',
      'Kéo thanh tiêu đề để di chuyển cửa sổ, rồi thả nó về giữa màn hình.',
    ],
    right: [
      'Thanh tiêu đề là của Windows: thu nhỏ, phóng to, đóng ở góc phải trên; bảng Snap hiện khi rê chuột lên nút phóng to.',
      'Thanh bên trái bắt đầu ngay dưới thanh tiêu đề: không có hàng trống, không có ba chấm màu.',
      'Windows 11: nền cửa sổ hơi trong, ánh màu hình nền (Mica), góc bo tròn nhỏ. Windows 10: nền đặc. Cửa sổ vừa trong màn hình.',
    ],
    help: [{ kind: 'window', tab: 'overview', label: 'Mở cửa sổ chính' }, SHOT_MAIN],
    captures: ['main'],
    checks: ['mainWindow'],
    covers: ['W2.6', 'W5.2', 'W5.3'],
  },
  {
    id: 'm1-look',
    part: 1,
    title: 'Chữ, chủ đề và giao diện',
    do: [
      'Bấm “Mở Cài đặt › Hiển thị” của app.',
      'Ở Chủ đề, chọn Kính Nhịp, rồi Bàn điều khiển, rồi Mặc định.',
      'Ở Giao diện, chọn Sáng, rồi Tối, rồi Theo Windows.',
    ],
    right: [
      'Chữ là Segoe UI (gọn, giống chữ của Windows), không phải Arial; dấu tiếng Việt rõ.',
      'Kính Nhịp: chữ Inter. Bàn điều khiển: mọi chữ đều đều nhau như máy đánh chữ.',
      'Sáng và Tối: thanh tiêu đề, nền kính của cửa sổ và popover đổi theo. Theo Windows: theo chế độ ứng dụng của Windows.',
    ],
    help: [{ kind: 'window', tab: 'settings', group: 'display', label: 'Mở Cài đặt › Hiển thị' }],
    captures: ['main'],
    covers: ['W5.1', 'W2.8'],
  },
  {
    id: 'm1-tray-card',
    part: 1,
    title: 'Khay hệ thống',
    do: [
      'Ở Cài đặt › Hiển thị của app, tìm thẻ “Khay hệ thống”.',
      'Đổi “Dịch vụ trên icon khay” sang Codex (nếu Codex mờ thì giữ nguyên), rồi “Số trên icon khay” sang Tuần.',
      'Bấm “Chụp góc khay sau 3 giây” và để chuột trên icon khay.',
      'Đổi lại như lúc đầu (Claude, 5 giờ).',
    ],
    right: [
      'Thẻ “Khay hệ thống” nằm dưới thẻ Hiển thị, có hai dòng “Dịch vụ trên icon khay” và “Số trên icon khay”.',
      'Con số và tooltip của icon đổi theo lựa chọn. Không chỗ nào trong cửa sổ ghi “thanh menu”.',
    ],
    help: [{ kind: 'window', tab: 'settings', group: 'display', label: 'Mở Cài đặt › Hiển thị' }, SHOT_TRAY],
    captures: ['main'],
    checks: ['tray'],
    covers: ['W5.6'],
  },
  {
    id: 'm1-services',
    part: 1,
    title: 'Dịch vụ, Keychain và thanh cuộn',
    do: [
      'Bấm “Mở Cài đặt › Dịch vụ”, cuộn xuống tới phần biến môi trường.',
      'Bấm icon khay mở popover, nhìn phần trên cùng của nó.',
    ],
    right: [
      'Không có dòng “Quyền Keychain”. Popover không có dải báo về Keychain.',
      'Đường dẫn và tên biến như ANTHROPIC_API_KEY hiện bằng chữ đều nhau (Cascadia Mono hoặc Consolas).',
      'Thanh cuộn mảnh và mờ; nội dung không giật sang ngang khi thanh cuộn hiện. Hàng tab Cài đặt (Chung … Nâng cao) không có thanh cuộn.',
    ],
    help: [{ kind: 'window', tab: 'settings', group: 'services', label: 'Mở Cài đặt › Dịch vụ' }],
    captures: ['main', 'popover'],
    covers: ['W5.7', 'W5.4', 'W5.1'],
  },
  {
    id: 'm1-words',
    part: 1,
    title: 'Chữ tiếng Anh, tiếng Việt và tên máy',
    do: [
      'Bấm “Mở Cài đặt › Chung”. Ở Ngôn ngữ chọn English.',
      'Xem tab General, rồi bấm tab Display, rồi Notifications.',
      'Ở General, Language, chọn lại Tiếng Việt. Xem lại Chung, Hiển thị, Thông báo.',
    ],
    right: [
      'English: “This PC’s name”, “Match Windows”, “Windows notifications”.',
      'Tiếng Việt: “Tên máy này”, “Theo Windows”, “Thông báo Windows”.',
      'Tên máy mặc định giống tên trong Cài đặt Windows › Hệ thống › Giới thiệu (About).',
    ],
    help: [{ kind: 'window', tab: 'settings', group: 'general', label: 'Mở Cài đặt › Chung' }],
    captures: ['main'],
    checks: ['machine'],
    covers: ['W5.8', 'W2.9'],
  },
  {
    id: 'm1-transparency',
    part: 1,
    title: 'Tắt hiệu ứng trong suốt',
    do: [
      'Bấm “Mở Cài đặt › Màu”, tắt “Hiệu ứng trong suốt” (Transparency effects).',
      'Bấm vào cửa sổ chính của app, rồi bấm icon khay mở popover.',
      'Bật lại “Hiệu ứng trong suốt”. Bấm vào cửa sổ chính và mở popover lần nữa.',
    ],
    right: [
      'Khi tắt: popover và cửa sổ chính mất lớp kính mờ, nền đặc.',
      'Khi bật lại: lớp kính trở lại.',
    ],
    help: [{ kind: 'page', page: 'colors', label: 'Mở Cài đặt › Màu' }, { kind: 'window', tab: 'overview', label: 'Mở cửa sổ chính' }],
    captures: ['main', 'popover'],
    checks: ['material'],
    covers: ['W2.7'],
  },
  {
    id: 'm1-contrast',
    part: 1,
    title: 'Chủ đề tương phản',
    do: [
      'Bấm “Mở Cài đặt › Chủ đề tương phản”. Chọn Bầu trời đêm (Night sky), bấm Áp dụng. Bấm vào cửa sổ chính, rồi mở popover.',
      'Chọn Sa mạc (Desert), bấm Áp dụng, làm lại như trên.',
      'Chọn Không (None), bấm Áp dụng.',
    ],
    right: [
      'Thẻ, hộp thoại và menu có viền rõ; ô nhập có viền; thanh hạn mức vẫn thấy phần màu đã dùng.',
      'Không còn lớp kính; viền mạnh hơn.',
    ],
    help: [{ kind: 'page', page: 'contrast', label: 'Mở Cài đặt › Chủ đề tương phản' }, { kind: 'window', tab: 'overview', label: 'Mở cửa sổ chính' }],
    captures: ['main', 'popover'],
    checks: ['material'],
    covers: ['W5.5', 'W2.8'],
  },
  {
    id: 'm1-webview',
    part: 1,
    title: 'Chuột phải và phím tắt',
    do: [
      'Bấm “Mở Cài đặt › Chung”. Bấm chuột phải vào chỗ trống trong cửa sổ.',
      'Bấm chuột phải vào ô “Tên máy này”, rồi bấm Esc.',
      'Bấm vào chỗ trống, rồi lần lượt bấm: F5, Ctrl+R, Ctrl+F, Ctrl+P, F12, Ctrl+Shift+I, Alt+mũi tên trái. Nếu có gì mở ra, bấm Esc.',
      'Trong ô “Tên máy này”: Ctrl+A, Ctrl+C, rồi Ctrl+V (dán lại đúng chữ cũ).',
    ],
    right: [
      'Chuột phải ở chỗ trống: không có menu. Trong ô chữ: có Cắt, Sao chép, Dán.',
      'Các phím kia không làm gì: trang không tải lại, không có ô tìm, không có hộp thoại in. Sao chép và dán vẫn chạy.',
    ],
    help: [{ kind: 'window', tab: 'settings', group: 'general', label: 'Mở Cài đặt › Chung' }, SHOT_MAIN],
    covers: ['W2.11'],
  },
  {
    id: 'm1-open',
    part: 1,
    title: 'Mở liên kết và thư mục',
    do: [
      'Bấm lần lượt bốn nút bên dưới; mỗi nút mở một chương trình. Nhìn rồi đóng nó lại.',
      'Trong app, cái nào có thì mở: ở Kỹ năng, liên kết GitHub của một kỹ năng; ở Chỉ dẫn, một file; ở Cài đặt › Dịch vụ, nút mở một biến môi trường; ở Codex, “Mở lại trang đăng nhập” khi đang đăng nhập.',
    ],
    right: [
      'Trang phát hành và liên kết kỹ năng mở trong trình duyệt. Hai thư mục sao lưu mở trong File Explorer.',
      'File chỉ dẫn mở trong Notepad. Biến môi trường mở trong VS Code đúng dòng (nếu có VS Code), không thì Notepad.',
      'Cài đặt thông báo mở Cài đặt Windows › Thông báo.',
    ],
    help: [
      { kind: 'open', what: 'releases', label: 'Trang phát hành' },
      { kind: 'open', what: 'backup', label: 'Thư mục sao lưu' },
      { kind: 'open', what: 'mcpBackups', label: 'Thư mục sao lưu MCP' },
      { kind: 'open', what: 'notifySettings', label: 'Cài đặt thông báo' },
    ],
    covers: ['W2.12'],
  },
  {
    id: 'm1-end',
    part: 1,
    title: 'Hết phần 1',
    do: [
      'Bấm “Ghi gói chẩn đoán”. App ghi một file .zip lên Màn hình (Desktop).',
      'Dừng ở đây cũng được: kéo file đó vào chat là xong.',
      'Làm tiếp phần 2 (đăng nhập, thông báo, mở khi đăng nhập; khoảng 25 phút): bấm “Sang phần 2”.',
    ],
    right: ['File .zip đã có trên Màn hình, tên dạng diag-ngày.zip.'],
    end: 1,
    covers: [],
  },

  // ===== Phần 2: Lõi (M2) =====
  {
    id: 'm2-start',
    part: 2,
    title: 'Phần 2: tài khoản Claude',
    do: [
      'Nếu Claude Code trên máy chưa đăng nhập: mở Windows Terminal, gõ claude, đăng nhập theo hướng dẫn, rồi gõ /exit.',
      'Mở tab Claude của app. Nếu app mới giữ một tài khoản, bấm Thêm tài khoản và đăng nhập tài khoản thứ hai. Đóng trình duyệt khi xong.',
    ],
    right: ['Tab Claude có ít nhất hai tài khoản, mỗi tài khoản đọc được hạn mức.'],
    help: [{ kind: 'window', tab: 'claude', label: 'Mở tab Claude' }],
    checks: ['claudeStore'],
    covers: ['W3.1'],
  },
  {
    id: 'm2-autostart',
    part: 2,
    title: 'Mở khi đăng nhập',
    do: ['Bấm “Mở Cài đặt › Chung”, bật “Mở khi đăng nhập”.'],
    right: ['Công tắc bật, không báo lỗi.'],
    help: [{ kind: 'window', tab: 'settings', group: 'general', label: 'Mở Cài đặt › Chung' }],
    checks: ['autostart'],
    covers: ['W4.1'],
  },
  {
    id: 'm2-startup-off',
    part: 2,
    title: 'Tắt ở Cài đặt Windows',
    do: [
      'Bấm “Mở Ứng dụng khởi động”, tắt Quota Station ở đó.',
      'Quay lại Cài đặt › Chung của app: dòng “Mở khi đăng nhập” báo đang chờ. Bấm nút của dòng đó.',
      'Bật lại công tắc trong app. Tắt nó, rồi bật lại lần nữa.',
    ],
    right: [
      'Nút của dòng đang chờ mở trang Ứng dụng khởi động của Windows.',
      'Bật từ app: trang Ứng dụng khởi động hiện Quota Station bật. Cuối bước này công tắc đang bật.',
    ],
    help: [{ kind: 'page', page: 'startup', label: 'Mở Ứng dụng khởi động' }, { kind: 'window', tab: 'settings', group: 'general', label: 'Mở Cài đặt › Chung' }],
    checks: ['autostart'],
    covers: ['W4.2'],
  },
  {
    id: 'm2-toast',
    part: 2,
    title: 'Thông báo có nút',
    do: [
      'Bấm “Gửi thông báo thử”. Khi thông báo hiện ở góc phải dưới, bấm nút “Sang …” trên nó.',
      'Bấm “Gửi thông báo thử” lần nữa; lần này bấm vào phần chữ của thông báo, không bấm nút.',
    ],
    right: [
      'Thông báo ghi “Quota Station” (không phải PowerShell), có tiêu đề, nội dung và các nút.',
      'Bấm nút: popover mở ở phần chuyển tài khoản. Bấm phần chữ: cửa sổ chính mở với bảng chuông thông báo.',
    ],
    help: [{ kind: 'toast', label: 'Gửi thông báo thử' }],
    captures: ['toast'],
    covers: ['W4.3'],
  },
  {
    id: 'm2-action-center',
    part: 2,
    title: 'Trung tâm thông báo',
    do: [
      'Bấm “Gửi thông báo thử”, không bấm gì, chờ thông báo tự ẩn (khoảng 5 đến 10 giây).',
      'Bấm Windows + N, bấm một nút trên thông báo của Quota Station ở đó. Bấm Esc.',
      'Mở cửa sổ chính, bấm chuông.',
    ],
    right: [
      'Nút trong trung tâm thông báo không làm gì; đó là điều đã biết trước.',
      'Thông báo đó vẫn còn trong chuông của app, và bấm được ở đó.',
    ],
    help: [{ kind: 'toast', label: 'Gửi thông báo thử' }],
    covers: ['W4.4'],
  },
  {
    id: 'm2-notify-off',
    part: 2,
    title: 'Tắt thông báo',
    do: [
      'Bấm “Mở Cài đặt › Thông báo”, tắt Quota Station trong danh sách ứng dụng.',
      'Trong app: Cài đặt › Thông báo, bấm nút kiểm lại quyền. Bấm nút mở cài đặt của nó.',
      'Ở Cài đặt Windows, tắt công tắc Thông báo trên cùng (cho mọi app), rồi kiểm lại trong app lần nữa.',
      'Bật lại cả hai.',
    ],
    right: [
      'App báo thông báo đang tắt khi tắt, đang bật khi bật; nút của nó mở trang Thông báo của Windows.',
    ],
    help: [{ kind: 'page', page: 'notifications', label: 'Mở Cài đặt › Thông báo' }, { kind: 'window', tab: 'settings', group: 'notify', label: 'Mở Cài đặt › Thông báo của app' }],
    captures: ['main'],
    covers: ['W4.5'],
  },
  {
    id: 'm2-rotation',
    part: 2,
    title: 'Xoay vòng khi app mở',
    do: [
      'Ở tab Claude, bật Xoay vòng (giữ luật đang có).',
      'Để Xoay vòng bật tới cuối phần 2; app tự lưu mỗi lần chạy.',
    ],
    right: ['Không có cửa sổ đen (console) nào nháy lên.'],
    help: [{ kind: 'window', tab: 'claude', label: 'Mở tab Claude' }],
    checks: ['rotation'],
    cannot: ({ model }) => (model.claudeAccounts.accounts.length < 2 ? 'App cần giữ hai tài khoản Claude cho bước này, nên bước này tự bỏ qua.' : null),
    covers: ['W4.6', 'W4.7'],
  },
  {
    id: 'm2-backup',
    part: 2,
    title: 'Sao lưu',
    do: [
      'Mở tab Sao lưu, xem thư mục, bấm “Sao lưu bây giờ”.',
      'Nếu dùng OneDrive: bấm mở thư mục sao lưu, bấm phải một gói › Giải phóng dung lượng (Free up space). Quay lại app, mở lại tab Sao lưu.',
    ],
    right: [
      'Thư mục là OneDrive\\Quota Station Agents, hoặc Documents\\Quota Station Agents khi máy không có OneDrive.',
      'Gói vừa giải phóng hiện là chưa tải về, và biểu tượng đám mây của nó trong File Explorer vẫn còn.',
    ],
    help: [{ kind: 'window', tab: 'backup', label: 'Mở tab Sao lưu' }],
    captures: ['main'],
    covers: ['W4.8', 'W4.9'],
  },
  {
    id: 'm2-diag',
    part: 2,
    title: 'Thông tin chẩn đoán',
    do: ['Bấm “Mở Cài đặt › Chung”, ở Cập nhật bấm mở Thông tin chẩn đoán. Đóng nó sau khi xem.'],
    right: ['Dòng thứ hai bắt đầu bằng Windows 11 (hoặc 10), phiên bản và build. Không dòng nào có C:\\Users\\tên-máy.'],
    help: [{ kind: 'window', tab: 'settings', group: 'general', label: 'Mở Cài đặt › Chung' }, SHOT_MAIN],
    checks: ['system'],
    covers: ['W4.10'],
  },
  {
    id: 'm2-claude-switch',
    part: 2,
    title: 'Chuyển tài khoản Claude',
    do: [
      'Trong popover, chuyển Claude sang tài khoản kia.',
      'Mở một Windows Terminal mới, gõ: claude -p "hi" rồi Enter. Chờ Claude trả lời, rồi đóng Terminal.',
      'Chuyển lại tài khoản cũ.',
    ],
    right: ['Chuyển không báo lỗi; claude trong Terminal mới chạy được với tài khoản vừa chuyển.'],
    checks: ['claudeStore'],
    cannot: ({ model }) => (model.claudeAccounts.accounts.length < 2 ? 'App cần giữ hai tài khoản Claude cho bước này, nên bước này tự bỏ qua.' : null),
    covers: ['W3.1'],
  },
  {
    id: 'm2-claude-busy',
    part: 2,
    title: 'Chuyển khi Claude đang chạy',
    do: [
      'Mở Windows Terminal, dán lệnh bên dưới (nút Chép), Enter. Lệnh chạy khoảng 1 đến 2 phút.',
      'Trong lúc lệnh chạy, chuyển tài khoản Claude trong popover hai lần.',
      'Chờ lệnh chạy xong, đóng Terminal.',
    ],
    right: ['Không bị đăng xuất; lệnh chạy hết.'],
    help: [{ kind: 'copy', text: () => '1..15 | % { claude -p hi }', label: 'Chép lệnh' }],
    checks: ['claudeStore'],
    skip: 'Hết thời gian: bỏ qua',
    cannot: ({ model }) => (model.claudeAccounts.accounts.length < 2 ? 'App cần giữ hai tài khoản Claude cho bước này, nên bước này tự bỏ qua.' : null),
    covers: ['W3.3'],
  },
  {
    id: 'm2-claude-folder',
    part: 2,
    title: 'Một tài khoản trong thư mục riêng',
    do: [
      'Mở Windows Terminal, dán lệnh bên dưới (nút Chép), Enter.',
      (c) => `Lệnh chạy claude bằng tài khoản số ${spareSlot(c.model) ?? '?'}, trong thư mục riêng của nó.`,
    ],
    right: ['Claude trả lời; tài khoản đang dùng trong popover không đổi.'],
    help: [{ kind: 'copy', text: (c) => run(c, `claude run ${spareSlot(c.model) ?? 2} -- -p hi`), label: 'Chép lệnh' }],
    checks: ['claudeSessions', 'claudeStore'],
    cannot: ({ model }) => (spareSlot(model) === null ? 'App cần một tài khoản Claude khác tài khoản đang dùng, nên bước này tự bỏ qua.' : null),
    covers: ['W3.4'],
  },
  {
    id: 'm2-claude-found',
    part: 2,
    title: 'App tìm thấy Claude Code',
    do: [
      'Mở tab Kỹ năng, rồi tab Plugin.',
      'Ở tab Claude, bấm Thêm tài khoản, rồi bấm Huỷ.',
    ],
    right: ['Hai tab Kỹ năng và Plugin mở được. Thêm tài khoản không báo thiếu Claude Code.'],
    help: [{ kind: 'window', tab: 'skills', label: 'Mở tab Kỹ năng' }],
    covers: ['W3.5'],
  },
  {
    id: 'm2-secrets',
    part: 2,
    title: 'Bí mật của app',
    do: [
      'Ở tab MCP, nếu chưa có MCP nào có token: thêm một MCP có token (ví dụ một máy chủ MCP đang dùng). Đã có thì thôi.',
    ],
    right: ['MCP hiện trong danh sách, token được giữ. Sau bước đăng xuất ở cuối, nó vẫn còn.'],
    help: [{ kind: 'window', tab: 'mcp', label: 'Mở tab MCP' }],
    checks: ['secrets'],
    skip: 'Không dùng MCP có token: bỏ qua',
    covers: ['W3.6'],
  },
  {
    id: 'm2-codex',
    part: 2,
    title: 'Codex: hai tài khoản ChatGPT',
    do: [
      'Ở tab Codex, thêm một tài khoản ChatGPT (đăng nhập trong trình duyệt), rồi một tài khoản nữa bằng mã thiết bị (device code).',
      'Tắt app ChatGPT, chuyển qua lại giữa hai tài khoản. Mở app ChatGPT, chuyển lần nữa.',
      'Bắt đầu thêm tài khoản rồi bấm Huỷ. Xoá một tài khoản vừa thêm. Nếu có file codex-*.json, nhập nó.',
      'Mở dòng “Tự đánh thức ChatGPT”, bật nó cho một tài khoản, bấm Chạy ngay.',
    ],
    right: [
      'Chuyển được; hoặc app báo Codex đang dùng thông tin đăng nhập, thoát Codex rồi thử lại, và không mất tài khoản nào.',
      'Không có cửa sổ đen (console) nào nháy lên.',
    ],
    help: [{ kind: 'window', tab: 'codex', label: 'Mở tab Codex' }],
    checks: ['codexAccounts', 'port1455'],
    skip: 'Hết thời gian: bỏ qua',
    cannot: noCodex,
    covers: ['W3.7'],
  },
  {
    id: 'm2-codex-keyring',
    part: 2,
    title: 'Codex giữ đăng nhập ở Credential Manager',
    do: [
      'Bấm “Mở config.toml của Codex”. Thêm dòng bên dưới (nút Chép) vào cuối file, lưu (Ctrl+S).',
      'Trong app, chuyển tài khoản Codex.',
      'Xoá dòng vừa thêm, lưu lại.',
    ],
    right: ['App từ chối chuyển và nói lý do là chỗ giữ đăng nhập của Codex.'],
    help: [
      { kind: 'page', page: 'codexConfig', label: 'Mở config.toml của Codex' },
      { kind: 'copy', text: () => 'cli_auth_credentials_store = "keyring"', label: 'Chép dòng' },
    ],
    checks: ['codexConfig'],
    skip: 'Hết thời gian: bỏ qua',
    cannot: noCodex,
    covers: ['W3.8'],
  },
  {
    id: 'm2-codex-procs',
    part: 2,
    title: 'Codex đang chạy',
    do: [
      'Mở Windows Terminal, gõ codex rồi Enter, để nó chạy. Mở app ChatGPT.',
      'Ở tab Codex của app, bấm Khởi động lại.',
      'Thoát codex trong Terminal (Ctrl+C hai lần).',
    ],
    right: ['Tab Codex hiện số phiên đang chạy, app ChatGPT và tiến trình nền; Khởi động lại chạy được.'],
    help: [{ kind: 'window', tab: 'codex', label: 'Mở tab Codex' }],
    skip: 'Hết thời gian: bỏ qua',
    cannot: noCodex,
    covers: ['W3.9'],
  },
  {
    id: 'm2-agy',
    part: 2,
    title: 'Antigravity (agy)',
    do: ['Mở popover, nhìn dòng Antigravity.'],
    right: ['Dòng Antigravity đọc được hạn mức, và không có trình duyệt nào tự mở.'],
    cannot: ({ model }) =>
      model.snapshot.agy.status === 'notSetUp' ? 'Máy này chưa có agy, nên bước này tự bỏ qua. Không cần cài gì thêm.' : null,
    covers: ['W3.10'],
  },
  {
    id: 'm2-env',
    part: 2,
    title: 'Biến môi trường',
    do: [
      'Mở Windows Terminal (PowerShell), dán lệnh đặt biến (nút Chép), Enter.',
      'Trong app: Cài đặt › Dịch vụ, bấm kiểm lại.',
      'Dán lệnh gỡ biến (nút Chép), Enter. Kiểm lại trong app lần nữa.',
    ],
    right: [
      'Khi đặt: ANTHROPIC_API_KEY hiện ở hai chỗ, biến người dùng và PowerShell, không hiện giá trị.',
      'Khi gỡ: dòng đó biến mất. Claude Code dùng lại đăng nhập như cũ.',
    ],
    help: [
      {
        kind: 'copy',
        text: () =>
          `setx ANTHROPIC_API_KEY qs-test; if (!(Test-Path $PROFILE)) { New-Item -ItemType File -Force $PROFILE | Out-Null }; Add-Content $PROFILE '$env:ANTHROPIC_API_KEY = "qs-test"'`,
        label: 'Chép lệnh đặt biến',
      },
      {
        kind: 'copy',
        text: () =>
          `[Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', $null, 'User'); (Get-Content $PROFILE) | ? { $_ -notmatch 'qs-test' } | Set-Content $PROFILE`,
        label: 'Chép lệnh gỡ biến',
      },
      { kind: 'window', tab: 'settings', group: 'services', label: 'Mở Cài đặt › Dịch vụ' },
    ],
    skip: 'Hết thời gian: bỏ qua',
    covers: ['W3.11'],
  },
  {
    id: 'm2-credman',
    part: 2,
    title: 'Claude giữ đăng nhập ở Credential Manager',
    do: [
      'Mở Windows Terminal (PowerShell), dán lệnh 1 (nút Chép), Enter, đăng nhập lại trong trình duyệt, rồi gõ /exit.',
      'Trong app, chuyển tài khoản Claude rồi chuyển lại.',
      'Dán lệnh 2 (nút Chép), Enter: xoá mục của Claude Code trong Credential Manager và đăng nhập lại như thường. Gõ /exit.',
    ],
    right: ['App vẫn đọc được hạn mức và chuyển được, cả khi Claude Code giữ đăng nhập trong Credential Manager và khi quay lại file.'],
    help: [
      { kind: 'copy', text: () => '$env:CLAUDE_CODE_FORCE_WINDOWS_CREDMAN = "1"; claude /login', label: 'Chép lệnh 1' },
      {
        kind: 'copy',
        text: () =>
          `Remove-Item Env:CLAUDE_CODE_FORCE_WINDOWS_CREDMAN; cmdkey /list | Select-String 'Claude Code-credentials\\S*' -AllMatches | % { $_.Matches.Value } | % { cmdkey /delete:$_ }; claude /login`,
        label: 'Chép lệnh 2',
      },
    ],
    checks: ['claudeStore'],
    skip: 'Hết thời gian: bỏ qua',
    cannot: ({ model }) => (model.claudeAccounts.accounts.length < 2 ? 'App cần giữ hai tài khoản Claude cho bước này, nên bước này tự bỏ qua.' : null),
    covers: ['W3.2'],
  },
  {
    id: 'm2-restart',
    part: 2,
    title: 'Đăng xuất rồi đăng nhập lại',
    do: [
      'Lưu việc đang làm ở các app khác.',
      'Bấm Start › biểu tượng tài khoản › Đăng xuất (Sign out). Đăng nhập lại.',
      'Đừng mở Quota Station: nó phải tự chạy. Khung hướng dẫn tự mở lại ở bước tiếp.',
    ],
    right: ['Sau khi đăng nhập, icon khay tự hiện mà không cần bấm gì, và khung này mở lại.'],
    checks: ['autostart'],
    restart: true,
    covers: ['W4.1', 'W3.6', 'W4.6'],
  },
  {
    id: 'm2-after',
    part: 2,
    title: 'Sau khi đăng nhập lại',
    do: [
      'Mở popover: các tài khoản vẫn đọc được hạn mức.',
      'Ở tab MCP, MCP có token vẫn còn token.',
      'Ở tab Claude, tắt Xoay vòng nếu lúc đầu nó tắt. Ở Cài đặt › Chung, tắt “Mở khi đăng nhập” nếu không muốn giữ.',
    ],
    right: ['Không tài khoản nào đòi đăng nhập lại; MCP vẫn còn token.'],
    help: [{ kind: 'window', tab: 'claude', label: 'Mở tab Claude' }],
    checks: ['autostart', 'secrets', 'rotation', 'claudeStore'],
    covers: ['W3.6', 'W4.6', 'W4.7'],
  },
  {
    id: 'm2-end',
    part: 2,
    title: 'Xong',
    do: ['Bấm “Ghi gói chẩn đoán”. App ghi file .zip lên Màn hình (Desktop), thay file của phần 1.', 'Kéo file đó vào chat là xong phiên.'],
    right: ['File .zip đã có trên Màn hình, tên dạng diag-ngày.zip.'],
    end: 2,
    covers: [],
  },
];

/** A line of a step, its words for this PC. */
export const lineText = (line: Line, c: Ctx): string => (typeof line === 'function' ? line(c) : line);

/**
 * What the guide reads from the model at the end of a step, for the zip:
 * counts, states and kinds, never a name, an email or a path.
 */
export function facts(model: Model, session: SessionView) {
  const claude = model.claudeAccounts.accounts;
  const folder = model.backup.folder;
  return {
    firstRun: model.firstRun,
    language: model.language,
    theme: model.theme,
    themePack: model.themePack,
    tray: { follows: model.menubar, shows: model.menubarWindow },
    claude: {
      status: model.snapshot.claude.status,
      kept: claude.length,
      inUse: claude.find((a) => a.active)?.slot ?? null,
      on: claude.filter((a) => a.enabled).length,
      signInAgain: claude.filter((a) => a.status === 'reloginRequired').length,
      program: session.claudeProgram,
      rotation: model.claudeAccounts.rotation,
      canLogIn: model.claudeAccounts.claude,
    },
    codex: {
      present: model.codex.present,
      status: model.snapshot.codex.status,
      chatgpt: model.codex.accounts.filter((a) => a.kind === 'chatgpt').length,
      refusal: model.codex.refusal,
    },
    agy: { status: model.snapshot.agy.status },
    launchAtLogin: model.app.launchAtLogin,
    notifications: model.alerts.permission,
    unanswered: model.alerts.unanswered,
    backup: {
      folder: /onedrive/i.test(folder) ? 'OneDrive' : /documents/i.test(folder) ? 'Documents' : 'other',
      driveThere: model.backup.icloud,
      packs: model.backup.packs?.length ?? null,
      notDownloaded: model.backup.packs?.filter((p) => p.state === 'cloud').length ?? null,
    },
    env: model.env.found.map((f) => `${f.name} · ${f.place}`),
  };
}
