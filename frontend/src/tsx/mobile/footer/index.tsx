import * as React from 'react';
import style from '../../../css/mobile/footer.module.css';
import { useSearchParams } from 'react-router-dom';

export enum FooterOption {
    Home = '',
    Notification = 'notification',
	Editor = 'editor',
	Chat = 'chat',
	Account = 'account',
};

export function useCurrentFooter(): FooterOption {
	let [search_params, _] = useSearchParams();
	let footer = search_params.get('footer');
	if (footer == 'editor') {
		return FooterOption.Editor;
	} else if (footer == 'notification') {
		return FooterOption.Notification;
	} else if (footer == 'chat') {
		return FooterOption.Chat;
	} else if (footer == 'account') {
		return FooterOption.Account;
	}
	return FooterOption.Home;
}

export function Footer(): JSX.Element {
	return <div className={`footer ${style.footer}`}>
		<IconBlock icon="🏠" current_option={FooterOption.Home} />
		<IconBlock icon="🔔" current_option={FooterOption.Notification} />
		<IconBlock icon="✏️" current_option={FooterOption.Editor} />
		<IconBlock icon="🗨️" current_option={FooterOption.Chat} />
		<IconBlock icon="🐷" current_option={FooterOption.Account} />
	</div>;
}

function IconBlock(props: { icon: string, current_option: FooterOption }): JSX.Element {
	const footer_option = useCurrentFooter();
	let [search_params, setSearchParams] = useSearchParams();
	let is_current = footer_option == props.current_option;

	function onClick(): void {
		if (is_current) {
			return;
		}
		if (props.current_option == FooterOption.Home) {
			search_params.delete('footer');
		} else {
			search_params.set('footer', props.current_option);
		}
		setSearchParams(search_params);
	}

	return <div
		className={is_current ? `${style.iconBlockSelected} ${style.iconBlock}` : style.iconBlock}
		onClick={onClick}>
		<div className={style.icon}>
			{props.icon}
		</div>
	</div>;
}
