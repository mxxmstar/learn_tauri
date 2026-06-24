#ifndef BCMTOOL_H
#define BCMTOOL_H

#include <QObject>
#include <QThread>
#if defined(_WIN32)
#ifdef _WIN32_WINNT
#undef _WIN32_WINNT
#define _WIN32_WINNT 0x0600
#endif
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#include <sys/stat.h>

#include "console.h"
#include "bcm_rpc.h"


class BcmTool : public QObject
{
    Q_OBJECT
signals:
    void consolePrint(const QString& str, PrintLevel level = PrintLevel::Info);
    void signal_progress(const QString& action, int progress, int finish);  // finish 0:执行中, 1:执行成功, -1:执行失败
    void versionInfo(const QString& str);
    void healthCheckSend(uint16_t pid);
    void configPair(const QString& pair);
    void writeConfigSignal(const QString name, const QString val);

public:
    BcmTool(QObject *parent = nullptr);
    ~BcmTool();

    void reboot(const QString deviceIP);
    void readConfig(const QString deviceIP);
    void writeConfig(const QString deviceIP, const QString name, const QString value);
    void healthCheck(const QString deviceIP, uint16_t pid);
    void getVersion(const QString deviceIP);
    void fullInstall(const QString fileName, const QByteArray fileBytes, const QString deviceIP, const QString hostIP);

private:
    WSADATA wsaData_;
};

#endif // BCMTOOL_H
