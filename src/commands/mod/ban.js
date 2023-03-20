const { SlashCommandBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('ban')
        .setDescription('Ban a user'),
    async execute(client, message, args) {
        if (!message.member.hasPermission('BAN_MEMBERS')) {
            message.channel.send("You don't have permission to use that command.");
        }
        else {
            try {
                let bannedMember = await message.guild.members.ban(args);
                if (bannedMember)
                    console.log(bannedMember.tag + " was banned.");
            }
            catch (err) {
                console.log(err);
            }
        }
    }
}