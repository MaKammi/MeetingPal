import { mkdir, writeFile } from 'node:fs/promises';

const models = [
  { id: 'whisper-small', name: 'Whisper Small · multilingual', task: 'transcription', license: 'MIT', repo: 'ggerganov/whisper.cpp', names: ['ggml-small.bin'] },
  { id: 'whisper-turbo', name: 'Whisper Large v3 Turbo', task: 'transcription', license: 'MIT', repo: 'ggerganov/whisper.cpp', names: ['ggml-large-v3-turbo.bin'] },
  { id: 'qwen-text', name: 'Qwen3 4B Instruct · Q4', task: 'analysis', license: 'Apache-2.0', repo: 'unsloth/Qwen3-4B-Instruct-2507-GGUF', names: ['Qwen3-4B-Instruct-2507-Q4_K_M.gguf'] },
  { id: 'qwen-vision', name: 'Qwen3 VL 4B Instruct · Q4', task: 'vision', license: 'Apache-2.0', repo: 'Qwen/Qwen3-VL-4B-Instruct-GGUF', names: ['Qwen3VL-4B-Instruct-Q4_K_M.gguf', 'mmproj-Qwen3VL-4B-Instruct-Q8_0.gguf'] },
];
const catalog=[];
for (const model of models) {
  const response=await fetch(`https://huggingface.co/api/models/${model.repo}?blobs=true`);
  if (!response.ok) throw new Error(`${model.repo}: ${response.status}`);
  const info=await response.json();
  const files=model.names.map(filename=> {
    const file=info.siblings.find(f=>f.rfilename===filename);
    if (!file?.lfs?.sha256) throw new Error(`Missing LFS SHA-256: ${model.repo}/${filename}`);
    return { filename, bytes:file.size, sha256:file.lfs.sha256, url:`https://huggingface.co/${model.repo}/resolve/${info.sha}/${filename}` };
  });
  catalog.push({id:model.id,name:model.name,task:model.task,license:model.license,source:`https://huggingface.co/${model.repo}`,files});
}
await mkdir('resources',{recursive:true});
await writeFile('resources/models.json',JSON.stringify(catalog,null,2)+'\n');
console.log('Pinned model revisions, sizes and SHA-256 checksums for',catalog.length,'models.');
