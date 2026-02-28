import { readFileSync, writeFileSync } from 'fs';

const indexPath = './src/index.html';
const content = readFileSync(indexPath, 'utf-8');

const match = content.match(/main\.js\?v=(\d+)\.(\d+)\.(\d+)/);
if (match) {
  const major = parseInt(match[1]);
  const minor = parseInt(match[2]);
  const patch = parseInt(match[3]);
  
  const newVersion = `${major}.${minor}.${patch + 1}`;
  const newContent = content.replace(
    /main\.js\?v=\d+\.\d+\.\d+/,
    `main.js?v=${newVersion}`
  );
  
  writeFileSync(indexPath, newContent);
  console.log(`Updated version to ${newVersion}`);
} else {
  console.log('Version pattern not found');
}
