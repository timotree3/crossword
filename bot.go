package main

import (
	"io/ioutil"
	"log"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/bwmarrin/discordgo"
)

var (
	nyc *time.Location
)

const (
	TODAY_TOPIC = `Discussion of today's (%s) crossword and sharing of times.`
	YESTERDAY_TOPIC = `Discussion of yesterday's (%s) crossword and sharing of times.`
)
//weekendReleaseTime: 6:00 PM
//weekdayReleaseTime: 10:00 PM

func init() {
  nyc, err := time.LoadLocation("America/New_York")
  if err != nil {
    log.Fatalln(err)
  }
}

func main() {
	Token, err := ioutil.ReadFile("token")
	if err != nil {
		log.Fatalln("error retrieving token from file:", err)
	}
	token := strings.TrimSpace(string(Token))
	// Create a new Discord session using the provided bot token.
	dg, err := discordgo.New("Bot " + token)
	if err != nil {
		log.Fatalln("error creating Discord session,", err)
	}

	dg.AddHandler(guildJoin)
	// Register the messageCreate func as a callback for MessageCreate events.
	dg.AddHandler(messageCreate)

	// Open a websocket connection to Discord and begin listening.
	err = dg.Open()
	if err != nil {
		log.Fatalln("error opening connection,", err)
	}

	if _, err := dg.User("@me"); err != nil {
		// Login unsuccessful
		log.Fatalln(err)
	}

	// Wait here until CTRL-C or other term signal is received.
	log.Println("Bot is now running.  Press CTRL-C to exit.")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	// Cleanly close down the Discord session.
	dg.Close()
}

//Initialization code
func guildJoin(s *discordgo.Session, event *discordgo.GuildCreate) {

	if event.Guild.Unavailable {
		return
	}

	log.Printf("guild: %#v\n", event.Guild.Name)
	// Does #today exist?
	today, ok := findChannel(event.Guild, "today")
	if !ok {
		// Create #today
		// today = newToday
	}
	now := time.Now().In(nyc)
	// Is #today up to date?
	if today.Topic == fmt.Sprintf(TODAY_TOPIC, now.Format("1/2/2006 (MST)")) {

	}

}

// This function will be called (due to AddHandler above) every time a new
// message is created on any channel that the autenticated bot has access to.
func messageCreate(s *discordgo.Session, m *discordgo.MessageCreate) {
	log.Println("messageCreate")
	c, err := s.Channel(m.ChannelID)
	if err != nil {
		log.Println(err)
	}
	g, err := s.Guild(c.GuildID)
	if err != nil {
		log.Println(err)
	}
	log.Println(g.Name)

	// Ignore all messages created by the bot itself
	// This isn't required in this specific example but it's a good practice.
	if m.Author.ID == s.State.User.ID {
		return
	}
	// If the message is "ping" reply with "Pong!"
	if m.Content == "ping" {
		send(s, m.ChannelID, "Pong!")
	}

	// If the message is "pong" reply with "Ping!"
	if m.Content == "pong" {
		send(s, m.ChannelID, "Ping!")
	}
}

func send(s *discordgo.Session, channelID string, content string) (*discordgo.Message, error) {
	return s.ChannelMessageSend(channelID, "\u200B"+content)
}

func findChannel(guild *discordgo.Guild, name string) (*discordgo.Channel, bool) {
	for _, channel := range event.Guild.Channels {
		if channel.Name == name {
			return channel, true
		}
	}
	return nil, false
}

//-------FUNCTIONS FOR RELEASE TIMES-------
func releaseTime(t time.Time) time.Time {
	if t.Weekday() == time.Saturday || t.Weekday() == time.Sunday {
		return time.Date(t.Year(), t.Month(), t.Day(), WEEKEND_HOUR, 0, 0, 0, t.Location()).Add(release_delay)
	} else {
		return time.Date(t.Year(), t.Month(), t.Day(), WEEKDAY_HOUR, 0, 0, 0, t.Location()).Add(release_delay)
	}
}

func nextRelease(t time.Time) time.Time {
	if releaseTime(t).After(t) {
		// it has yet to be released today
		return releaseTime(t)
	} else {
		// return tomorrow's release time
		return releaseTime(t.Add(24 * time.Hour))
	}
}

func lastRelease(t time.Time) time.Time {
	if releaseTime(t).Before(t) {
		// it was already released today
		return releaseTime(t)
	} else {
		// return yesterday's release time
		return releaseTime(t.Add(24 * -time.Hour))
	}
}
/*
At any given time there are two things we need to be able to know.

When the next crossword will be released:
	func releaseTime(day of week) -> 6:00PM|10:00PM

	//we need to know: now, today's day of week
	if releaseTime(today) is after now:
		crossword will be released at releaseTime(today)
	else:
		crossword will be released at releaseTime(tomorrow)

When was the most recent crossword released:
	if releaseTime(today) is before now:
		crossword was released at releaseTime(today)
	else:
		crossword was released at releaseTime(yesterday)
*/
//--------------
