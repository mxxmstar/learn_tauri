//! SSH 远程文件管理功能验证示例（第六版）。
//!
//! 这个示例不是正式产品功能的一部分，
//! 它的目标只有一个：在开发阶段快速验证 SSH / SFTP 主链路和第六版新增能力是否可用。
//!
//! 当前验证流程：
//! 1. 探测主机指纹；
//! 2. 按严格指纹校验建立 SSH 连接；
//! 3. 读取远程目录；
//! 4. 读取首项属性；
//! 5. 验证第三版目录操作：新建目录 -> 重命名 -> 删除；
//! 6. 验证第四版文本回写：保存文本 -> 检查属性大小 -> 删除测试文件；
//! 7. 验证第五版目录递归下载；
//! 8. 验证第五版目录递归上传；
//! 9. 验证第六版非空目录递归删除；
//! 10. 如有传入本地文件路径，再验证单文件上传；
//! 11. 最后断开 SSH 会话。
//!
//! 运行示例：
//! ```powershell
//! cargo run --manifest-path src-tauri\Cargo.toml --example ssh_validate -- 192.168.66.117 22 pyl pyl /home/pyl/tmp
//! ```

use std::{
    fs as std_fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use learn_tauri_lib::ssh::{
    self,
    types::{SshConnectRequest, SshHostProbeRequest, SshSessionInfo},
};

/// 使用 Tokio 运行时承载异步 SSH / SFTP 调用。
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 5 {
        eprintln!(
            "参数不足。用法：ssh_validate <host> <port> <username> <password> [path] [local_upload_path] [remote_upload_dir]"
        );
        std::process::exit(2);
    }

    let host = args[1].clone();
    let port = args[2].parse::<u16>().unwrap_or(22);
    let username = args[3].clone();
    let password = args[4].clone();
    let path = args.get(5).cloned().unwrap_or_else(|| "/".to_string());
    let local_upload_path = args.get(6).cloned();
    let remote_upload_dir = args.get(7).cloned().unwrap_or_else(|| path.clone());

    println!("开始验证 SSH 连接：{}:{} @ {}", host, port, username);

    let probe_result = ssh::ssh_probe_host(SshHostProbeRequest {
        host: host.clone(),
        port,
    })
    .await;

    if !probe_result.success || probe_result.data.is_none() {
        eprintln!(
            "SSH 主机指纹探测失败：{}",
            probe_result.error.unwrap_or_else(|| "未知错误".to_string())
        );
        std::process::exit(1);
    }

    let fingerprint = probe_result
        .data
        .expect("主机指纹探测成功但没有返回数据")
        .fingerprint;
    println!("已探测到主机指纹：{}", fingerprint);

    let connect_result = ssh::ssh_connect(SshConnectRequest {
        host,
        port,
        username,
        password,
        expected_host_fingerprint: Some(fingerprint),
        initial_path: Some(path.clone()),
    })
    .await;

    if !connect_result.success || connect_result.data.is_none() {
        eprintln!(
            "SSH 连接失败：{}",
            connect_result.error.unwrap_or_else(|| "未知错误".to_string())
        );
        std::process::exit(1);
    }

    let session = connect_result
        .data
        .expect("SSH 连接成功但没有返回会话信息");
    print_session_info(&session);

    if let Err(error) = validate_directory_listing(&session, &path).await {
        eprintln!("目录与属性验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Err(error) = validate_v3_directory_ops(&session, &remote_upload_dir).await {
        eprintln!("第三版目录操作验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Err(error) = validate_v4_text_save(&session, &remote_upload_dir).await {
        eprintln!("第四版文本保存验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Err(error) = validate_v5_recursive_download(&session, &remote_upload_dir).await {
        eprintln!("第五版递归下载验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Err(error) = validate_v5_recursive_upload(&session, &remote_upload_dir).await {
        eprintln!("第五版递归上传验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Err(error) = validate_v6_recursive_delete(&session, &remote_upload_dir).await {
        eprintln!("第六版递归删除验证失败：{}", error);
        let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
        std::process::exit(1);
    }

    if let Some(local_upload_path) = local_upload_path {
        if let Err(error) = validate_upload(&session, &local_upload_path, &remote_upload_dir).await {
            eprintln!("上传验证失败：{}", error);
            let _ = ssh::ssh_disconnect(session.session_id.clone()).await;
            std::process::exit(1);
        }
    }

    let disconnect_result = ssh::ssh_disconnect(session.session_id).await;
    if disconnect_result.success {
        println!("SSH 会话已断开，验证结束。");
    } else {
        eprintln!(
            "断开 SSH 会话时出现错误：{}",
            disconnect_result
                .error
                .unwrap_or_else(|| "未知错误".to_string())
        );
    }
}

/// 打印会话信息，便于人工确认连接目标。
fn print_session_info(session: &SshSessionInfo) {
    println!(
        "连接成功：sessionId={} host={} port={} username={} currentPath={} fingerprint={}",
        session.session_id,
        session.host,
        session.port,
        session.username,
        session.current_path,
        session.host_fingerprint
    );
}

/// 验证目录读取和属性读取。
async fn validate_directory_listing(session: &SshSessionInfo, path: &str) -> Result<(), String> {
    let list_result = ssh::sftp_list_directory(session.session_id.clone(), path.to_string()).await;
    if !list_result.success || list_result.data.is_none() {
        return Err(list_result.error.unwrap_or_else(|| "目录读取失败".to_string()));
    }

    let entries = list_result.data.expect("目录读取成功但没有返回数据");
    println!("目录 `{}` 读取成功，共 {} 项。", path, entries.len());

    for entry in entries.iter().take(10) {
        println!(
            "- {} | type={} | dir={} | size={} | path={}",
            entry.name, entry.file_type, entry.is_dir, entry.size, entry.path
        );
    }

    if let Some(first_entry) = entries.first() {
        println!("继续验证第一项属性读取：{}", first_entry.path);
        let property_result =
            ssh::sftp_get_properties(session.session_id.clone(), first_entry.path.clone()).await;

        if !property_result.success || property_result.data.is_none() {
            return Err(
                property_result
                    .error
                    .unwrap_or_else(|| "属性读取失败".to_string()),
            );
        }

        let properties = property_result
            .data
            .expect("属性读取成功但没有返回数据");
        println!(
            "属性读取成功：name={} type={} permission={:?} size={}",
            properties.name, properties.file_type, properties.permission_text, properties.size
        );
    }

    Ok(())
}

/// 验证第三版新增的目录操作。
async fn validate_v3_directory_ops(
    session: &SshSessionInfo,
    parent_dir: &str,
) -> Result<(), String> {
    let now = current_unix_seconds()?;
    let create_name = format!("codex_ssh_v3_validate_{}", now);
    let rename_name = format!("{}_renamed", create_name);

    println!(
        "开始验证第三版目录操作：parent_dir={} create_name={} rename_name={}",
        parent_dir, create_name, rename_name
    );

    let create_result = ssh::sftp_create_directory(
        session.session_id.clone(),
        parent_dir.to_string(),
        create_name.clone(),
    )
    .await;

    if !create_result.success || create_result.data.is_none() {
        return Err(
            create_result
                .error
                .unwrap_or_else(|| "创建目录失败".to_string()),
        );
    }

    let created_path = create_result
        .data
        .expect("创建目录成功但没有返回数据")
        .path;
    println!("目录创建成功：{}", created_path);

    let rename_result = ssh::sftp_rename_path(
        session.session_id.clone(),
        created_path.clone(),
        rename_name.clone(),
    )
    .await;

    if !rename_result.success || rename_result.data.is_none() {
        return Err(
            rename_result
                .error
                .unwrap_or_else(|| "重命名目录失败".to_string()),
        );
    }

    let renamed_path = rename_result
        .data
        .expect("重命名成功但没有返回数据")
        .new_path;
    println!("目录重命名成功：{}", renamed_path);

    let delete_result = ssh::sftp_delete_path(session.session_id.clone(), renamed_path.clone()).await;
    if !delete_result.success || delete_result.data.is_none() {
        return Err(
            delete_result
                .error
                .unwrap_or_else(|| "删除目录失败".to_string()),
        );
    }

    println!("目录删除成功：{}", renamed_path);
    Ok(())
}

/// 验证第四版新增的“文本保存回远程”能力。
async fn validate_v4_text_save(
    session: &SshSessionInfo,
    parent_dir: &str,
) -> Result<(), String> {
    let now = current_unix_seconds()?;
    let remote_path = join_remote_path(parent_dir, &format!("codex_ssh_v4_validate_{}.txt", now));
    let text_content = format!(
        "Codex SSH V4 validate file\nunix_seconds={}\nmessage=save remote text from validation example\n",
        now
    );
    let expected_size = text_content.as_bytes().len() as u64;

    println!(
        "开始验证第四版文本保存：remote_path={} expected_size={}",
        remote_path, expected_size
    );

    let save_result = ssh::sftp_save_text_file(
        session.session_id.clone(),
        remote_path.clone(),
        text_content,
    )
    .await;

    if !save_result.success || save_result.data.is_none() {
        return Err(
            save_result
                .error
                .unwrap_or_else(|| "保存远程文本文件失败".to_string()),
        );
    }

    let save_info = save_result
        .data
        .expect("保存远程文本文件成功但没有返回数据");
    println!(
        "远程文本保存成功：path={} size={} duration_ms={}",
        save_info.remote_path, save_info.file_size, save_info.duration_ms
    );

    if save_info.file_size != expected_size {
        return Err(format!(
            "保存结果大小不符合预期：expected={} actual={}",
            expected_size, save_info.file_size
        ));
    }

    let property_result =
        ssh::sftp_get_properties(session.session_id.clone(), remote_path.clone()).await;
    if !property_result.success || property_result.data.is_none() {
        return Err(
            property_result
                .error
                .unwrap_or_else(|| "读取第四版测试文件属性失败".to_string()),
        );
    }

    let properties = property_result
        .data
        .expect("读取第四版测试文件属性成功但没有返回数据");
    println!(
        "第四版测试文件属性读取成功：type={} size={} permission={:?}",
        properties.file_type, properties.size, properties.permission_text
    );

    if properties.size != expected_size {
        return Err(format!(
            "远程属性大小不符合预期：expected={} actual={}",
            expected_size, properties.size
        ));
    }

    let delete_result = ssh::sftp_delete_path(session.session_id.clone(), remote_path.clone()).await;
    if !delete_result.success || delete_result.data.is_none() {
        return Err(
            delete_result
                .error
                .unwrap_or_else(|| "删除第四版测试文件失败".to_string()),
        );
    }

    println!("第四版测试文件删除成功：{}", remote_path);
    Ok(())
}

/// 验证第五版递归下载能力。
async fn validate_v5_recursive_download(
    session: &SshSessionInfo,
    parent_dir: &str,
) -> Result<(), String> {
    let now = current_unix_seconds()?;
    let remote_root_name = format!("codex_ssh_v5_download_src_{}", now);
    let remote_root_path = create_remote_validation_tree(session, parent_dir, &remote_root_name).await?;
    let local_base_dir = std::env::temp_dir().join(format!("codex_ssh_v5_download_target_{}", now));

    if local_base_dir.exists() {
        std_fs::remove_dir_all(&local_base_dir)
            .map_err(|error| format!("清理本地下载验证目录失败：{}", error))?;
    }
    std_fs::create_dir_all(&local_base_dir)
        .map_err(|error| format!("创建本地下载验证目录失败：{}", error))?;

    println!(
        "开始验证第五版递归下载：remote_root={} local_base_dir={}",
        remote_root_path,
        local_base_dir.display()
    );

    let download_result = ssh::sftp_download_directory_without_events(
        session.session_id.clone(),
        remote_root_path.clone(),
        local_base_dir.to_string_lossy().to_string(),
    )
    .await;

    if !download_result.success || download_result.data.is_none() {
        let _ = cleanup_remote_tree(session, &remote_root_path).await;
        let _ = std_fs::remove_dir_all(&local_base_dir);
        return Err(
            download_result
                .error
                .unwrap_or_else(|| "递归下载目录失败".to_string()),
        );
    }

    let result = download_result
        .data
        .expect("递归下载成功但没有返回数据");
    println!(
        "第五版递归下载成功：remote={} local={} files={} dirs={} bytes={}",
        result.remote_path,
        result.local_path,
        result.file_count,
        result.directory_count,
        result.total_bytes
    );

    if result.file_count != 3 || result.directory_count != 3 {
        let _ = cleanup_remote_tree(session, &remote_root_path).await;
        let _ = std_fs::remove_dir_all(&local_base_dir);
        return Err(format!(
            "递归下载统计结果不符合预期：files={} dirs={}",
            result.file_count, result.directory_count
        ));
    }

    let local_root_path = PathBuf::from(&result.local_path);
    ensure_local_file_content(
        &local_root_path.join("readme.txt"),
        "Codex SSH V5 recursive download validation",
    )?;
    ensure_local_file_content(
        &local_root_path.join("nested").join("data.json"),
        "\"version\": 5",
    )?;
    ensure_local_file_content(
        &local_root_path.join("nested").join("deeper").join("note.md"),
        "recursive-download",
    )?;

    cleanup_remote_tree(session, &remote_root_path).await?;
    std_fs::remove_dir_all(&local_base_dir)
        .map_err(|error| format!("清理本地下载验证目录失败：{}", error))?;
    println!("第五版递归下载验证完成并已清理测试数据。");
    Ok(())
}

/// 验证第五版递归上传能力。
async fn validate_v5_recursive_upload(
    session: &SshSessionInfo,
    parent_dir: &str,
) -> Result<(), String> {
    let now = current_unix_seconds()?;
    let local_root_path = std::env::temp_dir().join(format!("codex_ssh_v5_upload_src_{}", now));

    if local_root_path.exists() {
        std_fs::remove_dir_all(&local_root_path)
            .map_err(|error| format!("清理本地上传验证目录失败：{}", error))?;
    }

    create_local_validation_tree(&local_root_path)?;

    println!(
        "开始验证第五版递归上传：local_root={} remote_parent={}",
        local_root_path.display(),
        parent_dir
    );

    let upload_result = ssh::sftp_upload_directory_without_events(
        session.session_id.clone(),
        local_root_path.to_string_lossy().to_string(),
        parent_dir.to_string(),
    )
    .await;

    if !upload_result.success || upload_result.data.is_none() {
        let _ = std_fs::remove_dir_all(&local_root_path);
        return Err(
            upload_result
                .error
                .unwrap_or_else(|| "递归上传目录失败".to_string()),
        );
    }

    let result = upload_result
        .data
        .expect("递归上传成功但没有返回数据");
    println!(
        "第五版递归上传成功：local={} remote={} files={} dirs={} bytes={}",
        result.local_path,
        result.remote_path,
        result.file_count,
        result.directory_count,
        result.total_bytes
    );

    if result.file_count != 3 || result.directory_count != 3 {
        let _ = cleanup_remote_tree(session, &result.remote_path).await;
        let _ = std_fs::remove_dir_all(&local_root_path);
        return Err(format!(
            "递归上传统计结果不符合预期：files={} dirs={}",
            result.file_count, result.directory_count
        ));
    }

    validate_remote_file_size(
        session,
        &join_remote_path(&result.remote_path, "readme.txt"),
        "Codex SSH V5 recursive upload validation\nroot=true\n".as_bytes().len() as u64,
    )
    .await?;
    validate_remote_file_size(
        session,
        &join_remote_path(&result.remote_path, "nested/config.toml"),
        "name = \"codex\"\nversion = 5\n".as_bytes().len() as u64,
    )
    .await?;
    validate_remote_file_size(
        session,
        &join_remote_path(&result.remote_path, "nested/deeper/info.txt"),
        "recursive-upload=true\n".as_bytes().len() as u64,
    )
    .await?;

    cleanup_remote_tree(session, &result.remote_path).await?;
    std_fs::remove_dir_all(&local_root_path)
        .map_err(|error| format!("清理本地上传验证目录失败：{}", error))?;
    println!("第五版递归上传验证完成并已清理测试数据。");
    Ok(())
}

/// 验证第六版递归删除非空目录能力。
async fn validate_v6_recursive_delete(
    session: &SshSessionInfo,
    parent_dir: &str,
) -> Result<(), String> {
    let now = current_unix_seconds()?;
    let remote_root_name = format!("codex_ssh_v6_delete_src_{}", now);
    let remote_root_path = create_remote_validation_tree(session, parent_dir, &remote_root_name).await?;

    println!("开始验证第六版递归删除：remote_root={}", remote_root_path);

    let delete_result =
        ssh::sftp_delete_path(session.session_id.clone(), remote_root_path.clone()).await;

    if !delete_result.success || delete_result.data.is_none() {
        let _ = cleanup_remote_tree(session, &remote_root_path).await;
        return Err(
            delete_result
                .error
                .unwrap_or_else(|| "递归删除非空目录失败".to_string()),
        );
    }

    let list_result =
        ssh::sftp_list_directory(session.session_id.clone(), parent_dir.to_string()).await;

    if !list_result.success || list_result.data.is_none() {
        return Err(
            list_result
                .error
                .unwrap_or_else(|| "删除后读取父目录失败".to_string()),
        );
    }

    let entries = list_result
        .data
        .expect("删除后读取父目录成功但没有返回数据");

    if entries.iter().any(|entry| entry.path == remote_root_path) {
        return Err(format!("递归删除后目录仍然存在：{}", remote_root_path));
    }

    println!("第六版递归删除验证成功：{}", remote_root_path);
    Ok(())
}

/// 验证单文件上传能力。
async fn validate_upload(
    session: &SshSessionInfo,
    local_upload_path: &str,
    remote_upload_dir: &str,
) -> Result<(), String> {
    println!(
        "继续验证上传功能：local={} -> remote_dir={}",
        local_upload_path, remote_upload_dir
    );

    let upload_result = ssh::sftp_upload_file_without_events(
        session.session_id.clone(),
        local_upload_path.to_string(),
        remote_upload_dir.to_string(),
    )
    .await;

    if !upload_result.success || upload_result.data.is_none() {
        return Err(upload_result.error.unwrap_or_else(|| "上传失败".to_string()));
    }

    let info = upload_result.data.expect("上传成功但没有返回数据");
    println!(
        "上传成功：remote_path={} size={} duration_ms={}",
        info.remote_path, info.file_size, info.duration_ms
    );

    Ok(())
}

/// 在远程创建一个固定结构的测试目录树。
async fn create_remote_validation_tree(
    session: &SshSessionInfo,
    parent_dir: &str,
    root_name: &str,
) -> Result<String, String> {
    let create_root_result = ssh::sftp_create_directory(
        session.session_id.clone(),
        parent_dir.to_string(),
        root_name.to_string(),
    )
    .await;

    if !create_root_result.success || create_root_result.data.is_none() {
        return Err(
            create_root_result
                .error
                .unwrap_or_else(|| "创建远程测试根目录失败".to_string()),
        );
    }

    let root_path = create_root_result
        .data
        .expect("创建远程测试根目录成功但没有返回数据")
        .path;

    let nested_create = ssh::sftp_create_directory(
        session.session_id.clone(),
        root_path.clone(),
        "nested".to_string(),
    )
    .await;
    if !nested_create.success || nested_create.data.is_none() {
        return Err(
            nested_create
                .error
                .unwrap_or_else(|| "创建远程 nested 目录失败".to_string()),
        );
    }
    let nested_path = nested_create
        .data
        .expect("创建远程 nested 目录成功但没有返回数据")
        .path;

    let deeper_create = ssh::sftp_create_directory(
        session.session_id.clone(),
        nested_path.clone(),
        "deeper".to_string(),
    )
    .await;
    if !deeper_create.success || deeper_create.data.is_none() {
        return Err(
            deeper_create
                .error
                .unwrap_or_else(|| "创建远程 deeper 目录失败".to_string()),
        );
    }
    let deeper_path = deeper_create
        .data
        .expect("创建远程 deeper 目录成功但没有返回数据")
        .path;

    save_remote_text(
        session,
        &join_remote_path(&root_path, "readme.txt"),
        "Codex SSH V5 recursive download validation\nroot=true\n",
    )
    .await?;
    save_remote_text(
        session,
        &join_remote_path(&nested_path, "data.json"),
        "{\n  \"version\": 5,\n  \"scenario\": \"recursive-download\"\n}\n",
    )
    .await?;
    save_remote_text(
        session,
        &join_remote_path(&deeper_path, "note.md"),
        "# recursive-download\nthis file is created for validation\n",
    )
    .await?;

    Ok(root_path)
}

/// 在本地创建一个固定结构的测试目录树。
fn create_local_validation_tree(root_path: &Path) -> Result<(), String> {
    std_fs::create_dir_all(root_path.join("nested").join("deeper"))
        .map_err(|error| format!("创建本地验证目录失败：{}", error))?;

    std_fs::write(
        root_path.join("readme.txt"),
        "Codex SSH V5 recursive upload validation\nroot=true\n",
    )
    .map_err(|error| format!("写入本地 readme.txt 失败：{}", error))?;

    std_fs::write(
        root_path.join("nested").join("config.toml"),
        "name = \"codex\"\nversion = 5\n",
    )
    .map_err(|error| format!("写入本地 config.toml 失败：{}", error))?;

    std_fs::write(
        root_path.join("nested").join("deeper").join("info.txt"),
        "recursive-upload=true\n",
    )
    .map_err(|error| format!("写入本地 info.txt 失败：{}", error))?;

    Ok(())
}

/// 验证远程文件大小是否符合预期。
async fn validate_remote_file_size(
    session: &SshSessionInfo,
    remote_path: &str,
    expected_size: u64,
) -> Result<(), String> {
    let property_result =
        ssh::sftp_get_properties(session.session_id.clone(), remote_path.to_string()).await;

    if !property_result.success || property_result.data.is_none() {
        return Err(
            property_result
                .error
                .unwrap_or_else(|| format!("读取远程文件属性失败：{}", remote_path)),
        );
    }

    let properties = property_result
        .data
        .expect("读取远程文件属性成功但没有返回数据");

    if properties.size != expected_size {
        return Err(format!(
            "远程文件大小不符合预期：path={} expected={} actual={}",
            remote_path, expected_size, properties.size
        ));
    }

    Ok(())
}

/// 保存远程文本文件，并在失败时返回详细错误。
async fn save_remote_text(
    session: &SshSessionInfo,
    remote_path: &str,
    content: &str,
) -> Result<(), String> {
    let save_result = ssh::sftp_save_text_file(
        session.session_id.clone(),
        remote_path.to_string(),
        content.to_string(),
    )
    .await;

    if !save_result.success || save_result.data.is_none() {
        return Err(
            save_result
                .error
                .unwrap_or_else(|| format!("保存远程文本失败：{}", remote_path)),
        );
    }

    Ok(())
}

/// 递归清理远程目录树。
async fn cleanup_remote_tree(session: &SshSessionInfo, root_path: &str) -> Result<(), String> {
    let root_path = root_path.to_string();
    let mut directories_to_remove = vec![root_path.clone()];
    let mut stack = vec![root_path];

    while let Some(current_path) = stack.pop() {
        let list_result =
            ssh::sftp_list_directory(session.session_id.clone(), current_path.clone()).await;

        if !list_result.success || list_result.data.is_none() {
            return Err(
                list_result
                    .error
                    .unwrap_or_else(|| format!("读取远程目录失败：{}", current_path)),
            );
        }

        let entries = list_result
            .data
            .expect("读取远程目录成功但没有返回数据");

        for entry in entries {
            if entry.is_dir {
                directories_to_remove.push(entry.path.clone());
                stack.push(entry.path);
            } else {
                delete_remote_path(session, &entry.path).await?;
            }
        }
    }

    directories_to_remove.sort_by(|left, right| {
        right
            .matches('/')
            .count()
            .cmp(&left.matches('/').count())
            .then_with(|| right.cmp(left))
    });

    for directory_path in directories_to_remove {
        delete_remote_path(session, &directory_path).await?;
    }

    Ok(())
}

/// 删除远程单个路径。
async fn delete_remote_path(session: &SshSessionInfo, path: &str) -> Result<(), String> {
    let delete_result = ssh::sftp_delete_path(session.session_id.clone(), path.to_string()).await;
    if !delete_result.success || delete_result.data.is_none() {
        return Err(
            delete_result
                .error
                .unwrap_or_else(|| format!("删除远程路径失败：{}", path)),
        );
    }
    Ok(())
}

/// 验证本地文件是否存在并包含指定关键片段。
fn ensure_local_file_content(path: &Path, expected_fragment: &str) -> Result<(), String> {
    let content = std_fs::read_to_string(path)
        .map_err(|error| format!("读取本地验证文件失败：{}，错误：{}", path.display(), error))?;

    if !content.contains(expected_fragment) {
        return Err(format!(
            "本地验证文件内容不符合预期：{}，未找到片段：{}",
            path.display(),
            expected_fragment
        ));
    }

    Ok(())
}

/// 读取当前 Unix 秒级时间戳。
fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("读取系统时间失败：{}", error))
        .map(|duration| duration.as_secs())
}

/// 拼接远程路径，始终输出 POSIX 风格路径。
fn join_remote_path(parent_dir: &str, file_name: &str) -> String {
    if parent_dir == "/" {
        format!("/{}", file_name)
    } else {
        format!("{}/{}", parent_dir.trim_end_matches('/'), file_name)
    }
}
