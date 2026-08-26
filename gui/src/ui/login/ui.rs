use gpui::prelude::FluentBuilder;
use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px, rgb, size,
};
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputContentType};
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::{Disableable, Root, h_flex, v_flex, v_virtual_list};
use std::rc::Rc;

use super::{LoginView, Page, core};

const PRIMARY: u32 = 0x38bdf8;
const PRIMARY_DARK: u32 = 0x0284c7;
const BACKGROUND: u32 = 0xeaf6ff;
const SURFACE: u32 = 0xffffff;
const BORDER: u32 = 0xbae6fd;
const ACCOUNT_ROW_HEIGHT: f32 = 68.;
const ACCOUNT_LIST_MAX_HEIGHT: f32 = 300.;

pub fn render(
    view: &LoginView,
    window: &mut Window,
    cx: &mut Context<LoginView>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .bg(rgb(BACKGROUND))
        .child(view.title_bar.clone())
        .child(
            v_flex()
                .flex_1()
                .items_center()
                .bg(rgb(BACKGROUND))
                .when(view.current_page == Page::Register, |this| {
                    this.child(render_avatar(view, cx))
                })
                .child(render_auth_content(view, cx)),
        )
        .children(Root::render_dialog_layer(window, cx))
        .children(Root::render_notification_layer(window, cx))
        .children(Root::render_sheet_layer(window, cx))
}

fn render_avatar(view: &LoginView, cx: &mut Context<LoginView>) -> impl IntoElement {
    let mut avatar = div()
        .size(px(106.))
        .items_center()
        .justify_center()
        .rounded_full()
        .border_2()
        .border_color(rgb(BORDER))
        .overflow_hidden();

    if let Some(path) = &view.avatar_path {
        avatar = avatar.child(
            Avatar::new()
                .src(path.to_string())
                .size_full()
                .into_any_element(),
        );
    }

    let avatar = avatar.when(
        view.current_page == Page::Register && !view.button_loading,
        |this| {
            this.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| core::choose_avatar(view, cx)),
            )
        },
    );

    v_flex()
        .w_full()
        .h(px(148.))
        .flex_shrink_0()
        .items_center()
        .justify_end()
        .pb_3()
        .child(avatar)
}

fn render_auth_content(view: &LoginView, cx: &mut Context<LoginView>) -> impl IntoElement {
    let content = match view.current_page {
        Page::AccountSelection => v_flex()
            .w_full()
            .max_w(px(312.))
            .gap_3()
            .child(render_saved_accounts(view, cx))
            .child(render_account_selection_switchers(view, cx)),
        Page::Login | Page::Register => v_flex()
            .w_full()
            .max_w(px(312.))
            .gap_3()
            .child(render_form(view, cx)),
    };

    v_flex()
        .w_full()
        .flex_1()
        .items_center()
        .when(
            matches!(view.current_page, Page::AccountSelection | Page::Login),
            |this| this.justify_center(),
        )
        .px_8()
        .child(content)
}

fn render_saved_accounts(view: &LoginView, cx: &mut Context<LoginView>) -> impl IntoElement {
    let account_list_height =
        px((view.saved_sessions.len() as f32 * ACCOUNT_ROW_HEIGHT).min(ACCOUNT_LIST_MAX_HEIGHT));

    v_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(PRIMARY_DARK))
                .child("已登录账号"),
        )
        .child(
            h_flex()
                .w_full()
                .h(account_list_height)
                .min_h_0()
                .gap_1()
                .child(
                    v_virtual_list(
                        cx.entity().clone(),
                        "login-saved-accounts",
                        Rc::new(
                            view.saved_sessions
                                .iter()
                                .map(|_| size(px(312.), px(68.)))
                                .collect(),
                        ),
                        |view, visible_range, _, cx| {
                            visible_range
                                .map(|index| {
                                    let session = view.saved_sessions[index].clone();
                                    let username = if session.username.is_empty() {
                                        "未设置用户名".to_string()
                                    } else {
                                        session.username.clone()
                                    };
                                    let login_name = session.login_name.clone();
                                    let user_id = session.user_id.clone();
                                    h_flex()
                                        .id(("login-saved-account", index))
                                        .w_full()
                                        .gap_2()
                                        .p_2()
                                        .rounded(px(10.))
                                        .border_1()
                                        .border_color(rgb(BORDER))
                                        .bg(rgb(SURFACE))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(rgb(0xf0f9ff)))
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            core::select_saved_account(view, index, cx)
                                        }))
                                        .child(render_account_avatar(session.user_avatar.clone()))
                                        .child(
                                            v_flex()
                                                .min_w_0()
                                                .flex_1()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .text_color(rgb(0x1e293b))
                                                        .child(username),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x64748b))
                                                        .child(format!("登录名: {login_name}")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(rgb(0x94a3b8))
                                                        .child(format!("ID: {user_id}")),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(18.))
                                                .text_color(rgb(PRIMARY_DARK))
                                                .child("›"),
                                        )
                                })
                                .collect()
                        },
                    )
                    .track_scroll(&view.account_scroll_handler),
                )
                .child(
                    Scrollbar::vertical(&view.account_scroll_handler)
                        .mode(ScrollbarMode::Always)
                        .axis(ScrollbarAxis::Vertical),
                ),
        )
}

fn render_account_avatar(source: String) -> impl IntoElement {
    let mut avatar = div()
        .size(px(42.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(rgb(BORDER))
        .overflow_hidden();
    if source.is_empty() {
        avatar = avatar.child(div().text_size(px(20.)).child("🙂"));
    } else {
        avatar = avatar.child(Avatar::new().src(source).size_full().into_any_element());
    }
    avatar
}

fn render_form(view: &LoginView, cx: &mut Context<LoginView>) -> impl IntoElement {
    let fields = v_flex().gap_3();

    let fields = fields
        .child(render_text_input(
            Input::new(&view.login_name)
                .aria_label("登录名")
                .content_type(InputContentType::Username)
                .cleanable(true),
        ))
        .child(render_text_input(
            Input::new(&view.password)
                .aria_label("密码")
                .content_type(InputContentType::Password)
                .mask_toggle(),
        ));

    let fields = if view.current_page == Page::Register {
        fields.child(render_text_input(
            Input::new(&view.password_confirmation)
                .aria_label("确认密码")
                .content_type(InputContentType::NewPassword)
                .mask_toggle(),
        ))
    } else {
        fields
    };

    let action = match view.current_page {
        Page::Login => Button::new("login-submit")
            .label("登录")
            .primary()
            .w_full()
            .h(px(40.))
            .rounded(px(10.))
            .border_color(rgb(PRIMARY_DARK))
            .bg(rgb(PRIMARY))
            .text_color(rgb(0xffffff))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .loading(view.button_loading)
            .disabled(view.button_loading)
            .on_click(cx.listener(|view, _, _, cx| core::submit_login(view, cx))),
        Page::Register => Button::new("register-submit")
            .label("注册")
            .primary()
            .w_full()
            .h(px(40.))
            .rounded(px(10.))
            .border_color(rgb(PRIMARY_DARK))
            .bg(rgb(PRIMARY))
            .text_color(rgb(0xffffff))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .loading(view.button_loading)
            .disabled(view.button_loading)
            .on_click(cx.listener(|view, _, _, cx| core::submit_registration(view, cx))),
        Page::AccountSelection => unreachable!("account selection does not render the form"),
    };

    v_flex()
        .gap_4()
        .child(fields)
        .child(action)
        .child(render_page_switcher(view, cx))
}

fn render_text_input(input: Input) -> impl IntoElement {
    div()
        .w_full()
        .h(px(42.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(input.size_full().appearance(false))
}

fn render_page_switcher(view: &LoginView, cx: &mut Context<LoginView>) -> impl IntoElement {
    let switcher = match view.current_page {
        Page::Login => h_flex()
            .w_full()
            .justify_center()
            .gap_4()
            .when(!view.saved_sessions.is_empty(), |this| {
                this.child(
                    Button::new("show-account-selection")
                        .label("返回账号选择")
                        .link()
                        .text_color(rgb(PRIMARY_DARK))
                        .disabled(view.button_loading)
                        .on_click(
                            cx.listener(|view, _, _, cx| core::show_account_selection(view, cx)),
                        ),
                )
            })
            .child(
                Button::new("show-register")
                    .label("注册账号")
                    .link()
                    .text_color(rgb(PRIMARY_DARK))
                    .disabled(view.button_loading)
                    .on_click(cx.listener(|view, _, _, cx| core::show_registration(view, cx))),
            ),
        Page::Register => h_flex().w_full().justify_center().child(
            Button::new("show-login")
                .label("返回登录")
                .link()
                .text_color(rgb(PRIMARY_DARK))
                .disabled(view.button_loading)
                .on_click(cx.listener(|view, _, _, cx| core::show_login(view, cx))),
        ),
        Page::AccountSelection => h_flex(),
    };

    switcher
}

fn render_account_selection_switchers(
    view: &LoginView,
    cx: &mut Context<LoginView>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_3()
        .child(
            Button::new("show-password-login")
                .label("账号密码登录")
                .primary()
                .w_full()
                .h(px(40.))
                .rounded(px(10.))
                .border_color(rgb(PRIMARY_DARK))
                .bg(rgb(PRIMARY))
                .text_color(rgb(0xffffff))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .disabled(view.button_loading)
                .on_click(cx.listener(|view, _, _, cx| core::show_login(view, cx))),
        )
        .child(
            h_flex().w_full().justify_center().child(
                Button::new("show-register-from-account-selection")
                    .label("注册账号")
                    .link()
                    .text_color(rgb(PRIMARY_DARK))
                    .disabled(view.button_loading)
                    .on_click(cx.listener(|view, _, _, cx| core::show_registration(view, cx))),
            ),
        )
}
