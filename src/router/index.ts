import { createRouter, createWebHashHistory } from "vue-router";
import FilePage from "../views/FilePage.vue";
import SshFile from "../views/SshFile.vue";
import HelpPage from "../views/HelpPage.vue";
import SomeipPlayer from "../someip-player/SomeipPlayer.vue";
import DhcpServer from "../camera_tools/DhcpServer.vue";

/**
 * 路由配置。
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
    path: "/ssh",
    name: "ssh",
    component: SshFile,
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