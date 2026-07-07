<script setup lang="ts">
import { ref } from 'vue'
import { useStartupStore } from '@/stores/startup'

const store = useStartupStore()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'added'): void
}>()

const form = ref({
  name: '',
  command: '',
  args: '' as string,
  delaySeconds: 0,
  runAsAdmin: false,
})

const submit = async () => {
  if (!form.value.name.trim() || !form.value.command.trim()) return
  await store.add({
    name: form.value.name.trim(),
    command: form.value.command.trim(),
    args: form.value.args
      .split(/\s+/)
      .map((s) => s.trim())
      .filter(Boolean),
    delaySeconds: form.value.delaySeconds,
    runAsAdmin: form.value.runAsAdmin,
    workingDir: null,
  })
  emit('added')
}

const close = () => emit('close')
</script>

<template>
  <div class="modal-overlay" @click.self="close">
    <div class="modal-card">
      <h2 class="modal-title">添加自定义启动项</h2>
      <p class="modal-subtitle">注册到 HKCU\...\Run 启动项</p>

      <div class="form-row">
        <label>名称</label>
        <input v-model="form.name" class="input" placeholder="例如：VS Code" />
      </div>
      <div class="form-row">
        <label>命令（路径）</label>
        <input
          v-model="form.command"
          class="input"
          placeholder="例如：C:\Program Files\...\code.exe"
          autocomplete="off"
        />
      </div>
      <div class="form-row">
        <label>参数（空格分隔，可选）</label>
        <input v-model="form.args" class="input" placeholder="--new-window" />
      </div>
      <div class="form-row">
        <label>延迟启动（秒）</label>
        <input v-model.number="form.delaySeconds" type="number" class="input" min="0" />
      </div>
      <div class="form-row form-check">
        <label>
          <input v-model="form.runAsAdmin" type="checkbox" />
          以管理员权限运行
        </label>
      </div>

      <div class="modal-actions">
        <button class="btn btn-ghost" @click="close">取消</button>
        <button class="btn btn-primary" @click="submit">添加</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-title {
  margin: 0 0 4px 0;
  font-size: 20px;
}
.modal-subtitle {
  margin: 0 0 16px 0;
  font-size: 12px;
  color: var(--text-secondary);
}
.form-row {
  margin-bottom: 12px;
}
.form-row label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.form-check label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 24px;
}
</style>
