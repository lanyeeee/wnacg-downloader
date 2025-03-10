// @ts-nocheck
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

/** user-defined commands **/


export const commands = {
async greet(name: string) : Promise<string> {
    return await TAURI_INVOKE("greet", { name });
},
async getConfig() : Promise<Config> {
    return await TAURI_INVOKE("get_config");
},
async saveConfig(config: Config) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("save_config", { config }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async login(username: string, password: string) : Promise<Result<string, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("login", { username, password }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getUserProfile() : Promise<Result<UserProfile, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_user_profile") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async searchByKeyword(keyword: string, pageNum: number) : Promise<Result<SearchResult, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("search_by_keyword", { keyword, pageNum }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async searchByTag(tagName: string, pageNum: number) : Promise<Result<SearchResult, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("search_by_tag", { tagName, pageNum }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getComic(id: number) : Promise<Result<Comic, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_comic", { id }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getFavorite(shelfId: number, pageNum: number) : Promise<Result<GetFavoriteResult, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_favorite", { shelfId, pageNum }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async createDownloadTask(comic: Comic) : Promise<void> {
    await TAURI_INVOKE("create_download_task", { comic });
},
async pauseDownloadTask(comicId: number) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("pause_download_task", { comicId }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async resumeDownloadTask(comicId: number) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("resume_download_task", { comicId }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async cancelDownloadTask(comicId: number) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("cancel_download_task", { comicId }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
}
}

/** user-defined events **/


export const events = __makeEvents__<{
downloadSpeedEvent: DownloadSpeedEvent,
downloadTaskEvent: DownloadTaskEvent,
logEvent: LogEvent
}>({
downloadSpeedEvent: "download-speed-event",
downloadTaskEvent: "download-task-event",
logEvent: "log-event"
})

/** user-defined constants **/



/** user-defined types **/

export type Comic = { 
/**
 * 漫画id
 */
id: number; 
/**
 * 漫画标题
 */
title: string; 
/**
 * 封面链接
 */
cover: string; 
/**
 * 分类
 */
category: string; 
/**
 * 漫画有多少张图片
 */
imageCount: number; 
/**
 * 标签
 */
tags: Tag[]; 
/**
 * 简介
 */
intro: string; 
/**
 * 是否已下载
 */
isDownloaded?: boolean | null; 
/**
 * 图片列表
 */
imgList: ImgList }
export type ComicInFavorite = { 
/**
 * 漫画id
 */
id: number; 
/**
 * 漫画标题
 */
title: string; 
/**
 * 漫画封面链接
 */
cover: string; 
/**
 * 加入收藏的时间
 * 2025-01-04 16:04:34
 */
favoriteTime: string; 
/**
 * 这个漫画属于的书架
 */
shelf: Shelf; 
/**
 * 是否已下载
 */
isDownloaded: boolean }
export type ComicInSearch = { 
/**
 * 漫画id
 */
id: number; 
/**
 * 漫画标题(带html标签，用于显示匹配关键词)
 */
titleHtml: string; 
/**
 * 漫画标题
 */
title: string; 
/**
 * 封面链接
 */
cover: string; 
/**
 * 额外信息(209張圖片， 創建於2025-01-05 18:33:19)
 */
additionalInfo: string; 
/**
 * 是否已下载
 */
isDownloaded: boolean }
export type CommandError = { err_title: string; err_message: string }
export type Config = { cookie: string; downloadDir: string; enableFileLogger: boolean; downloadFormat: DownloadFormat }
export type DownloadFormat = "Jpeg" | "Png" | "Webp" | "Original"
export type DownloadSpeedEvent = { speed: string }
export type DownloadTaskEvent = { state: DownloadTaskState; comic: Comic; downloadedImgCount: number; totalImgCount: number }
export type DownloadTaskState = "Pending" | "Downloading" | "Paused" | "Cancelled" | "Completed" | "Failed"
export type GetFavoriteResult = { comics: ComicInFavorite[]; currentPage: number; totalPage: number; shelf: Shelf; shelves: Shelf[] }
export type ImgInImgList = { 
/**
 * 图片标题([01]、[001]，根据漫画总页数确定)
 */
caption: string; 
/**
 * 图片url(//img5.wnimg.ru/data/2826/33/01.jpg，缺https:前缀)
 * 最后一张图片为/themes/weitu/images/bg/shoucang.jpg，记得过滤
 */
url: string }
export type ImgList = ImgInImgList[]
export type JsonValue = null | boolean | number | string | JsonValue[] | Partial<{ [key in string]: JsonValue }>
export type LogEvent = { timestamp: string; level: LogLevel; fields: Partial<{ [key in string]: JsonValue }>; target: string; filename: string; line_number: number }
export type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
export type SearchResult = { comics: ComicInSearch[]; currentPage: number; totalPage: number; isSearchByTag: boolean }
export type Shelf = { 
/**
 * 书架id
 */
id: number; 
/**
 * 书架名称
 */
name: string }
export type Tag = { 
/**
 * 标签名
 */
name: string; 
/**
 * 标签链接
 */
url: string }
export type UserProfile = { 
/**
 * 用户名
 */
username: string; 
/**
 * 头像url
 */
avatar: string }

/** tauri-specta globals **/

import {
	invoke as TAURI_INVOKE,
	Channel as TAURI_CHANNEL,
} from "@tauri-apps/api/core";
import * as TAURI_API_EVENT from "@tauri-apps/api/event";
import { type WebviewWindow as __WebviewWindow__ } from "@tauri-apps/api/webviewWindow";

type __EventObj__<T> = {
	listen: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
	once: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
	emit: null extends T
		? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
		: (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> =
	| { status: "ok"; data: T }
	| { status: "error"; error: E };

function __makeEvents__<T extends Record<string, any>>(
	mappings: Record<keyof T, string>,
) {
	return new Proxy(
		{} as unknown as {
			[K in keyof T]: __EventObj__<T[K]> & {
				(handle: __WebviewWindow__): __EventObj__<T[K]>;
			};
		},
		{
			get: (_, event) => {
				const name = mappings[event as keyof T];

				return new Proxy((() => {}) as any, {
					apply: (_, __, [window]: [__WebviewWindow__]) => ({
						listen: (arg: any) => window.listen(name, arg),
						once: (arg: any) => window.once(name, arg),
						emit: (arg: any) => window.emit(name, arg),
					}),
					get: (_, command: keyof __EventObj__<any>) => {
						switch (command) {
							case "listen":
								return (arg: any) => TAURI_API_EVENT.listen(name, arg);
							case "once":
								return (arg: any) => TAURI_API_EVENT.once(name, arg);
							case "emit":
								return (arg: any) => TAURI_API_EVENT.emit(name, arg);
						}
					},
				});
			},
		},
	);
}
