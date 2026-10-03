<script setup lang="ts">
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import { snapshot, clockAction } from "../lib/store";
defineProps<{floating?:boolean}>();
const alerts=computed(()=>snapshot.value.settings.clockTools.alerts);
const busy=ref(false),error=ref("");
async function act(action:string,id?:number){busy.value=true;error.value="";try{await clockAction({action,id});}catch(e){error.value=typeof e==="string"?e:"操作未完成。";}finally{busy.value=false;}}
</script>
<template><section class="clock-reminder" :class="{'floating-reminder':floating}" role="alertdialog" aria-label="时间提醒" aria-live="assertive"><template v-if="alerts.length"><AppIcon name="alarm" :size="28"/><span class="extra-eyebrow">{{ alerts[0].missed ? '错过的提醒' : '时间到了' }}</span><h1>{{ alerts[0].label }}</h1><p>{{ new Date(alerts[0].firedAt).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit'}) }}{{ alerts.length>1 ? ` · 还有 ${alerts.length-1} 条提醒` : '' }}</p><div class="clock-tool-actions"><button class="clock-tool-button primary" :disabled="busy" @click="act('dismiss',alerts[0].id)">停止</button><button class="clock-tool-button" :disabled="busy" @click="act('snooze',alerts[0].id)">稍后 5 分钟</button><button v-if="alerts.length>1" class="clock-tool-button" :disabled="busy" @click="act('dismiss')">全部停止</button></div></template><p v-else>提醒已结束</p><p v-if="error" role="alert">{{ error }}</p></section></template>
