<script setup lang="ts">
import { computed, ref, watch } from "vue";
import VelaSelect from "./VelaSelect.vue";
import AppIcon from "./AppIcon.vue";
import { snapshot, selectNote, setNoteExpiry } from "../lib/store";
import { noteTitle } from "../lib/markdown";
const active = computed(() => snapshot.value.settings.note.notes.find(n => n.id === snapshot.value.settings.note.activeId));
const noteOptions = computed(() => snapshot.value.settings.note.notes.map(n => ({ value: String(n.id), label: noteTitle(n.text) })));
const duration = ref("never"); const hours = ref(24); const busy = ref(false); const error = ref("");
const options = [{ value: "never", label: "永不删除" }, { value: "24", label: "创建后 1 天" }, { value: "168", label: "创建后 7 天" }, { value: "720", label: "创建后 30 天" }, { value: "custom", label: "自定义时长" }];
watch(active, item => { const value = item?.deleteAfterHours; duration.value = value == null ? "never" : [24,168,720].includes(value) ? String(value) : "custom"; hours.value = value ?? 24; }, { immediate: true });
const dateLabel = (time: number) => new Date(time).toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
async function run(action: () => Promise<void>) { if (busy.value) return; busy.value = true; error.value = ""; try { await action(); } catch(e) { error.value = typeof e === "string" ? e : e instanceof Error ? e.message : "没有保存成功。"; } finally { busy.value = false; } }
function change(value: string) { duration.value = value; if (value !== "custom" && active.value) void run(() => setNoteExpiry(active.value!.id, value === "never" ? null : Number(value))); }
async function saveCustom() { if (active.value) await run(() => setNoteExpiry(active.value!.id, hours.value)); }
</script>
<template>
  <div class="settings-card stacked">
    <div class="card-row"><AppIcon class="card-icon" name="hourglass" :size="18" /><div class="card-text"><strong>自动删除</strong><span>每篇独立设置，从创建时间起计算；默认永不删除</span></div></div>
    <p v-if="error" class="info-bar" role="alert">{{ error }}</p>
    <div v-if="active" class="note-retention-controls" :class="{ 'is-busy': busy }">
      <label>便签<VelaSelect :model-value="String(active.id)" label="设置自动删除的便签" :options="noteOptions" @update:model-value="value => run(() => selectNote(Number(value)))" /></label>
      <label>保留时长<VelaSelect :model-value="duration" label="便签保留时长" :options="options" @update:model-value="change" /></label>
      <form v-if="duration === 'custom'" class="note-retention-custom" @submit.prevent="saveCustom"><label>小时<input v-model.number="hours" class="win-text-input" type="number" min="1" max="87600" step="1" required aria-label="自定义便签保留小时数" :disabled="busy" /></label><button class="win-button" :disabled="busy">保存时长</button></form>
      <p class="note-retention-date">创建于 {{ dateLabel(active.createdAt) }}<br />{{ active.deleteAfterHours === null ? '不会自动删除' : `预计删除：${dateLabel(active.createdAt + active.deleteAfterHours * 3600000)}` }}</p>
      <p class="note-retention-hint">到期后删除该篇内容；应用关闭期间到期的便签会在下次启动时清理。设置已超过创建时间的时长会立即删除。</p>
    </div><p v-else class="note-retention-date">还没有便签，请从桌面组件新建。</p>
  </div>
</template>
