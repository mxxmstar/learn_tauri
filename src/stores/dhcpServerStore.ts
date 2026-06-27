import { ref } from "vue";

/** DHCP 服务器运行状态（全局共享） */
const dhcpServerRunning = ref(false);

export function useDhcpServerStore() {
  /** 切换 DHCP 服务器运行状态 */
  const toggleDhcpServer = () => {
    dhcpServerRunning.value = !dhcpServerRunning.value;
  };

  /** 设置 DHCP 服务器运行状态 */
  const setDhcpServerRunning = (running: boolean) => {
    dhcpServerRunning.value = running;
  };

  return {
    dhcpServerRunning,
    toggleDhcpServer,
    setDhcpServerRunning,
  };
}