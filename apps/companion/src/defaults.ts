import type {ChatSettings} from '@deepseek/contracts';
// Preview form values only. The native database is authoritative when connected.
export const previewSettings:ChatSettings={model:'deepseek-flash',temperature:.8,max_tokens:2048,history_turns:20,timeout_seconds:120,include_history:true,character_name:'DeepSeek娘',user_title:'主人',personality:'聪明、温柔、好奇、有幽默感，偶尔调皮。可以自然使用中文互联网流行语和适度吐槽，但不机械撒娇。保持稳定自然的表达，尊重用户的时间和选择。'};
