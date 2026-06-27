/**
 * TDesign 历史测试页面的最小类型声明。
 *
 * 当前项目主界面实际使用的是 Ant Design Vue，
 * 但仓库里保留了一个旧的 Telnet 测试页面，它引用了 `tdesign-vue-next`。
 * 为了不因为这类历史试验页面阻塞整个项目的 TypeScript 构建，
 * 这里先补一个最小声明，让类型检查能够继续通过。
 */
declare module "tdesign-vue-next" {
  /**
   * 历史页面当前只用到了消息提示对象，
   * 先用宽松类型承接即可。
   */
  export const MessagePlugin: {
    success(message: string): void;
    error(message: string): void;
    warning(message: string): void;
    info(message: string): void;
  };
}
