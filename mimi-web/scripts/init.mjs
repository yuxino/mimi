import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

async function main() {
  console.log('=== 初始化 web-template 项目 ===');
  
  // 获取当前目录名作为项目名
  const rootDir = process.cwd();
  const defaultProjectName = path.basename(rootDir);
  
  // 读取package.json
  const packageJsonPath = path.join(rootDir, 'package.json');
  const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
  
  // 更新项目名
  if (packageJson.name === 'web-template') {
    packageJson.name = defaultProjectName;
    await fs.writeFile(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
    console.log(`已更新项目名为: ${defaultProjectName}`);
  }
  
  // 尝试获取git用户名作为author
  try {
    const { stdout: gitUserName } = await execFileAsync('git', ['config', 'user.name']);
    const { stdout: gitUserEmail } = await execFileAsync('git', ['config', 'user.email']);
    
    if (gitUserName.trim()) {
      packageJson.author = gitUserEmail.trim() 
        ? `${gitUserName.trim()} <${gitUserEmail.trim()}>`
        : gitUserName.trim();
      await fs.writeFile(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
      console.log(`已设置作者为: ${packageJson.author}`);
    }
  } catch {}
  
  // 更新README.md
  const readmePath = path.join(rootDir, 'README.md');
  let readmeContent = await fs.readFile(readmePath, 'utf8');
  readmeContent = readmeContent.replace(/^# web-template$/m, `# ${defaultProjectName}`);
  await fs.writeFile(readmePath, readmeContent);
  console.log('已更新README.md');

  console.log('\n✅ 初始化完成！');
  console.log(`项目名: ${defaultProjectName}`);
  console.log('请根据需要修改 .env.development 和 .env.production 中的配置');
}

main().catch((error) => {
  console.error('初始化失败:', error);
  process.exit(1);
});
