#ifndef BCM_RPC_H
#define BCM_RPC_H

#include <QObject>
#if defined(_WIN32)
#ifdef _WIN32_WINNT
#undef _WIN32_WINNT
#define _WIN32_WINNT 0x0600
#endif
#endif
#include <winsock2.h>
#include <ws2tcpip.h>

#include "bcm_common.h"
#include "bcm_flash.h"
#include "bcm_config.h"

class BcmRPC : public QObject
{
    Q_OBJECT

signals:
    void consolePrint(const QString& str, PrintLevel level = PrintLevel::Info);
    void finished();
    void versionInfo(const QString& str);
    void configPair(const QString& pair);

public:
    BcmRPC(const QString deviceIP, QObject *parent = nullptr);
    ~BcmRPC();
    void showConfig(CONFIG_RpcMsg *readMsg);

public slots:
    void fullInstall(uint32_t serverPort, const char* fileName, uint32_t fileSize, const char* ipv4_str_server);
    void getVersion();
    void healthCheck(uint16_t pid);
    void rebootFinished();
    void reboot();
    void readConfig();
    uint32_t writeConfigMsg(const QString name, const QString val, uint8_t* msg);
    void writeConfig(const QString name, const QString val);

private:
    const QString deviceIP_;
};

#endif // BCM_RPC_H
