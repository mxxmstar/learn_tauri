import { createRouter, createWebHashHistory } from "vue-router";
import FilePage from "../views/FilePage.vue";
import HelpPage from "../views/HelpPage.vue";
import SomeipPlayer from "../someip-player/SomeipPlayer.vue";
import DhcpServer from "../camera_tools/DhcpServer.vue";

/**
 * 路由配置
 *
 * 菜单结构：
 *   文件      → /file
 *   页面 ▾
 *     └── SOME/IP Player  → /someip-player
 *   工具 ▾
 *     └── DHCP Server     → /dhcp-server
 *   帮助      → /help
 */
const routes = [
  {
    path: "/",
    redirect: "/file",
  },
  {
    path: "/file",
    name: "file",
    component: FilePage,
  },
  {
    path: "/someip-player",
    name: "someip-player",
    component: SomeipPlayer,
  },
  {
    path: "/dhcp-server",
    name: "dhcp-server",
    component: DhcpServer,
  },
  {
    path: "/help",
    name: "help",
    component: HelpPage,
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;