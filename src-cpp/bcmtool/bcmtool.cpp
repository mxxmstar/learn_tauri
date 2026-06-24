#include "bcmtool.h"
#include "rpc_connect.h"
#include "bcm_flash.h"

BcmTool::BcmTool(QObject *parent)
    : QObject(parent)
{
    WSAStartup(MAKEWORD(2, 2), &wsaData_);
}

BcmTool::~BcmTool()
{
    WSACleanup();
}

void BcmTool::reboot(const QString deviceIP)
{
    QThread *rpcThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(rpcThread);

    connect(rpcThread, &QThread::started, bcmRPC, &BcmRPC::rebootFinished);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::finished, rpcThread, &QThread::quit);
    connect(rpcThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(rpcThread, &QThread::finished, rpcThread, &QObject::deleteLater);

    rpcThread->start();
}

void BcmTool::readConfig(const QString deviceIP)
{
    QThread *rpcThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(rpcThread);

    connect(rpcThread, &QThread::started, bcmRPC, &BcmRPC::readConfig);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::configPair, this, &BcmTool::configPair);
    connect(bcmRPC, &BcmRPC::finished, rpcThread, &QThread::quit);
    connect(rpcThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(rpcThread, &QThread::finished, rpcThread, &QObject::deleteLater);

    rpcThread->start();
}

void BcmTool::writeConfig(const QString deviceIP, const QString name, const QString value)
{
    QThread *rpcThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(rpcThread);

    connect(this, &BcmTool::writeConfigSignal, bcmRPC, &BcmRPC::writeConfig);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::finished, rpcThread, &QThread::quit);
    connect(rpcThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(rpcThread, &QThread::finished, rpcThread, &QObject::deleteLater);

    rpcThread->start();
    emit writeConfigSignal(name, value);
}

void BcmTool::healthCheck(const QString deviceIP, uint16_t pid)
{
    QThread *rpcThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(rpcThread);

    connect(this, &BcmTool::healthCheckSend, bcmRPC, &BcmRPC::healthCheck);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::finished, rpcThread, &QThread::quit);
    connect(rpcThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(rpcThread, &QThread::finished, rpcThread, &QObject::deleteLater);
    
    rpcThread->start();
    emit healthCheckSend(pid);
}

void BcmTool::getVersion(const QString deviceIP)
{
    QThread *rpcThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(rpcThread);

    connect(rpcThread, &QThread::started, bcmRPC, &BcmRPC::getVersion);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::versionInfo, this, &BcmTool::versionInfo);
    connect(bcmRPC, &BcmRPC::finished, rpcThread, &QThread::quit);
    connect(rpcThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(rpcThread, &QThread::finished, rpcThread, &QObject::deleteLater);
    
    rpcThread->start();
}

void BcmTool::fullInstall(const QString fileName, const QByteArray fileBytes, const QString deviceIP, const QString hostIP)
{
    QThread *serverThread = new QThread();
    FileTransferServer *fileTransferServer = new FileTransferServer(fileName, fileBytes, hostIP);
    fileTransferServer->moveToThread(serverThread);

    connect(fileTransferServer, &FileTransferServer::consolePrint, this, &BcmTool::consolePrint);
    connect(fileTransferServer, &FileTransferServer::signal_progress, this, &BcmTool::signal_progress);
    connect(fileTransferServer, &FileTransferServer::finished, serverThread, &QThread::quit);
    connect(serverThread, &QThread::started, fileTransferServer, &FileTransferServer::start);
    connect(serverThread, &QThread::finished, fileTransferServer, &QObject::deleteLater);
    connect(serverThread, &QThread::finished, serverThread, &QObject::deleteLater);

    serverThread->start();

    QThread *flashThread = new QThread();
    BcmRPC *bcmRPC = new BcmRPC(deviceIP);
    bcmRPC->moveToThread(flashThread);

    connect(fileTransferServer, &FileTransferServer::ready, bcmRPC, &BcmRPC::fullInstall);
    connect(bcmRPC, &BcmRPC::consolePrint, this, &BcmTool::consolePrint);
    connect(bcmRPC, &BcmRPC::finished, flashThread, &QThread::quit);
    connect(flashThread, &QThread::finished, bcmRPC, &QObject::deleteLater);
    connect(flashThread, &QThread::finished, flashThread, &QObject::deleteLater);
    
    flashThread->start();
}

