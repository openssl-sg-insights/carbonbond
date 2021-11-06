import { UserState } from '../global_state/user';
import { toastErr, useInputValue } from '../utils';
import useOnClickOutside from 'use-onclickoutside';
import { API_FETCHER, unwrap } from '../../ts/api/api';
import * as React from 'react';
import { toast } from 'react-toastify';
import DatePicker from 'react-datepicker';
import { ModalButton, ModalWindow } from '../components/modal_window';
import { isEmail } from '../../ts/regex_util';
import style from '../../css/header/login_modal.module.css';

import "react-datepicker/dist/react-datepicker.css";

enum Status {
	ForgetPassword,
	Login
};

function LoginStatus(
	props: {
		setLogining: (logining: boolean) => void,
		setStatus: (status: Status) => void
	}
): JSX.Element {
	let name = useInputValue('').input_props;
	let password = useInputValue('').input_props;
	const { setLogin } = UserState.useContainer();
	async function login_request(name: string, password: string): Promise<void> {
		try {
			let user = unwrap(await API_FETCHER.userQuery.login(name, password));
			props.setLogining(false);
			if (user) {
				setLogin({
					user_name: user.user_name,
					id: user.id,
					email: user.email,
					energy: user.energy,
				});
				toast('登入成功');
			} else {
				toast('帳號或密碼錯誤');
			}
		} catch (err) {
			toastErr(err);
		}
		return;
	}
	function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>): void {
		if (e.key == 'Enter') {
			login_request(name.value, password.value);
		} else if (e.key == 'Escape') {
			props.setLogining(false);
		}
	}

	const buttons: ModalButton[] = [
		{ text: '登入', handler: () => login_request(name.value, password.value) },
		{ text: '取消', handler: () => props.setLogining(false) }
	];

	function getBody(): JSX.Element {
		return <div className={style.loginModal}>
			<div>
				<input type="text" placeholder="😎 使用者名稱" autoFocus {...name} onKeyDown={onKeyDown} />
				<input type="password" placeholder="🔒 密碼" {...password} onKeyDown={onKeyDown} />
				<div className={style.fogetPassword} onClick={() => { props.setStatus(Status.ForgetPassword); }}>忘記密碼？</div>
			</div>
		</div>;
	}

	return <ModalWindow
		title="登入"
		body={getBody()}
		buttons={buttons}
		visible={true}
		setVisible={props.setLogining}
	/>;
}

function ForgetPasswordStatus(
	props: {
		setLogining: (logining: boolean) => void,
		setStatus: (status: Status) => void
	}
): JSX.Element {
	const [sent, setSent] = React.useState(false);
	let email = useInputValue('').input_props;
	async function reset_password_request(email: string): Promise<void> {
		try {
			if (!isEmail(email)) {
				throw '信箱格式異常';
			}
			unwrap(await API_FETCHER.userQuery.sendResetPasswordEmail(email));
			setSent(true);
		} catch (err) {
			toastErr(err);
		}
		return;
	}

	function getBody(): JSX.Element {
		return <div className={style.signupModal}>
			<input type="text" placeholder="😎 信箱" autoFocus {...email} />
			{
				sent ?
					<p>已寄出重置密碼信</p> :
					<p>&nbsp;</p>
			}
		</div>;
	}
	const buttons: ModalButton[] = [
		{ text: sent ? '再一次' : '重置密碼', handler: () => reset_password_request(email.value) },
		{ text: '返回', handler: () => props.setStatus(Status.Login) }
	];
	return <ModalWindow
		title="找回密碼"
		body={getBody()}
		buttons={buttons}
		visible={true}
		setVisible={props.setLogining}
	/>;
}

export function LoginModal(props: { setLogining: (logining: boolean) => void }): JSX.Element {
	let ref_all = React.useRef(null);
	const [status, setStatus] = React.useState<Status>(Status.Login);
	useOnClickOutside(ref_all, () => { props.setLogining(false); });
	switch (status) {
		case Status.Login:
			return <LoginStatus {...props} setStatus={setStatus} />;
		case Status.ForgetPassword:
			return <ForgetPasswordStatus {...props} setStatus={setStatus} />;
	}
}
// TODO: 發一個 API 問後端目前設定上是否允許自行註冊
export function SignupModal(props: { setSignuping: (signing: boolean) => void }): JSX.Element {
	const [certificate_data, setCertificateData] = React.useState<string | null>(null);
	let name = useInputValue('').input_props;
	let email = useInputValue('').input_props;
	const [birthday, setBirthday] = React.useState<Date>(new Date());
	const [gender, setGender] = React.useState<string>("男");

	let ref_all = React.useRef(null);
	useOnClickOutside(ref_all, () => props.setSignuping(false));
	async function signup_request(name: string, email: string): Promise<void> {
		try {
			if (!isEmail(email)) {
				throw '信箱格式異常';
			}
			if (certificate_data == null) {
				throw '證書格式異常';
			}
			unwrap(await API_FETCHER.userQuery.recordSignupApply(name, email, birthday.getFullYear(), birthday.getMonth() + 1, birthday.getDate(), gender, certificate_data.split(',')[1], false));
			console.log('申請已送出');
		} catch (err) {
			toastErr(err);
		}
		return;
	}

	function chooseCertificate(e: React.ChangeEvent<HTMLInputElement>): void {
		e.preventDefault();

		if (e.target.files == null) {
			return;
		}

		let reader = new FileReader();
		let file = e.target.files[0];

		reader.onloadend = () => {
			setCertificateData(reader.result as string); // 因為使用 readAsDataURL ，故 result 為字串
		};

		reader.readAsDataURL(file);
		return;
	}

	const buttons: ModalButton[] = [
		{
			text: '送出申請', handler: () => {
				signup_request(name.value, email.value);
				props.setSignuping(false);
			}
		},
		{ text: '取消', handler: () => props.setSignuping(false) }
	];

	function getBody(): JSX.Element {
		return <div className={style.signupModal}>
			<div className={style.data}>
				<span className={style.label}>姓名</span>
				<input type="text" placeholder="😎 姓名" autoFocus {...name} />
			</div>
			<div className={style.data}>
				<span className={style.label}>信箱</span>
				<input type="text" placeholder="😎 信箱" {...email} />
			</div>
			<div className={style.data}>
				<span className={style.label}>生日</span>
				<div>
					<DatePicker
						selected={birthday}
						onChange={(date: Date) => setBirthday(date)}
					/>
				</div>
			</div>
			<div className={style.data}>
				<span className={style.label}>性別</span>
				<div className={style.gender}>
					<input type="radio" name="gender" value="男" defaultChecked={gender === '男'} onChange={(e) => setGender(e.target.value)} />
					<div>男</div>
					<input type="radio" name="gender" value="女" defaultChecked={gender === '女'} onChange={(e) => setGender(e.target.value)} />
					<div>女</div>
				</div>
			</div>
			<div className={style.data}>
				<span className={style.label}>證書</span>
				<input
					type="file"
					accept="image/png, image/jpeg"
					onChange={chooseCertificate} />
			</div>
		</div>;
	}

	return <ModalWindow
		title="註冊"
		body={getBody()}
		buttons={buttons}
		visible={true}
		setVisible={props.setSignuping}
	/>;
}